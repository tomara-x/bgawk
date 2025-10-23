use bevy::prelude::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SizedSample, Stream, StreamConfig
};
use crossbeam_channel::{bounded, Receiver, Sender};
use fundsp::hacker32::*;
//use assert_no_alloc::*;

//#[cfg(debug_assertions)]
//#[global_allocator]
//static A: AllocDisabler = AllocDisabler;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, init_audio)
            .add_observer(set_out_device)
            .add_observer(set_in_device);
    }
}

/// fundsp::Slot::set this to any fundsp graph (with 0 inputs) to start playing it
/// make sure graph.outputs() == slot.outputs()
/// the outputs of this slot will be the number of channels the current output stream has
#[derive(Resource, DerefMut, Deref)]
pub struct AudioOutput(pub Slot);

/// input receiver (channel index, sample)
#[derive(Resource, Deref)]
pub struct AudioInputReceiver1(pub Receiver<(usize, f32)>);

/// identical to receiver 1 but a different channel (i.e. crossbeam channel)
/// this allows using one without affecting the other
#[derive(Resource, Deref)]
pub struct AudioInputReceiver2(pub Receiver<(usize, f32)>);

#[derive(Resource, Deref)]
pub struct InStreamConfig(pub Option<StreamConfig>);

#[derive(Resource, Deref)]
pub struct OutStreamConfig(pub Option<StreamConfig>);

/// trigger this event to start a new input stream (ending the current one)
/// the default (host/device/config) will be used for any field set to None
/// use `list_in_devices` to get a list of host/device indexes
#[derive(Event, Debug, Default)]
pub struct SetInDevice {
    pub host: Option<usize>,
    pub device: Option<usize>,
    pub channels: Option<u16>,
    pub sr: Option<u32>,
    pub buffer: Option<u32>,
}

/// trigger this event to start a new output stream (ending the current one)
/// the default (host/device/config) will be used for any field set to None
/// use `list_out_devices` to get a list of host/device indexes
#[derive(Event, Debug, Default)]
pub struct SetOutDevice {
    pub host: Option<usize>,
    pub device: Option<usize>,
    pub channels: Option<u16>,
    pub sr: Option<u32>,
    pub buffer: Option<u32>,
}

struct InStream(Option<Stream>);
struct OutStream(Option<Stream>);

pub fn init_audio(world: &mut World) {
    // dummy things
    let (slot, _) = Slot::new(Box::new(dc(0.)));
    world.insert_resource(AudioOutput(slot));
    world.insert_resource(InStreamConfig(None));
    world.insert_resource(OutStreamConfig(None));
    world.insert_non_send_resource(OutStream(None));
    world.insert_non_send_resource(InStream(None));
    let (_, r1) = bounded(1);
    world.insert_resource(AudioInputReceiver1(r1.clone()));
    world.insert_resource(AudioInputReceiver2(r1));
    // start default streams
    world.trigger(SetOutDevice {
        channels: Some(2),
        ..default()
    });
    world.trigger(SetInDevice::default());
}

fn set_out_device(
    trig: Trigger<SetOutDevice>,
    mut stream: NonSendMut<OutStream>,
    mut audio_output: ResMut<AudioOutput>,
    mut out_stream_config: ResMut<OutStreamConfig>,
) -> Result {
    let event = trig.event();
    let host = if let Some(h) = event.host {
        let host_id = cpal::ALL_HOSTS.get(h).ok_or("couldn't find that host")?;
        cpal::host_from_id(*host_id)?
    } else {
        cpal::default_host()
    };
    let device = if let Some(d) = event.device {
        let mut devices = host.output_devices()?;
        devices.nth(d).ok_or("couldn't find that device")?
    } else {
        host.default_output_device()
            .ok_or("no default output device")?
    };
    let default_config = device.default_output_config()?;
    let sample_format = default_config.sample_format();
    let mut config = default_config.config();

    if let Some(sr) = event.sr {
        config.sample_rate = cpal::SampleRate(sr);
    }
    if let Some(size) = event.buffer {
        config.buffer_size = cpal::BufferSize::Fixed(size);
    }
    if let Some(channels) = event.channels {
        config.channels = channels;
    }
    let mut net = Net::scalar(config.channels as usize, 0.);
    net.allocate();
    let (slot, slot_back) = Slot::new(Box::new(net));

    let s = match sample_format {
        cpal::SampleFormat::F32 => run_out::<f32>(&device, &config, slot_back),
        cpal::SampleFormat::I16 => run_out::<i16>(&device, &config, slot_back),
        cpal::SampleFormat::U16 => run_out::<u16>(&device, &config, slot_back),
        format => return Err(format!("unsupported sample format: {format}").into()),
    };
    if s.is_some() {
        stream.0 = s;
        audio_output.0 = slot;
        out_stream_config.0 = Some(config);
        Ok(())
    } else {
        Err(format!("couldn't start stream with given settings\n{event:?}").into())
    }
}

fn set_in_device(
    trig: Trigger<SetInDevice>,
    mut stream: NonSendMut<InStream>,
    mut audio_input_receiver1: ResMut<AudioInputReceiver1>,
    mut audio_input_receiver2: ResMut<AudioInputReceiver2>,
    mut in_stream_config: ResMut<InStreamConfig>,
) -> Result {
    let event = trig.event();
    let host = if let Some(h) = event.host {
        let host_id = cpal::ALL_HOSTS.get(h).ok_or("couldn't find that host")?;
        cpal::host_from_id(*host_id)?
    } else {
        cpal::default_host()
    };
    let device = if let Some(d) = event.device {
        let mut devices = host.input_devices()?;
        devices.nth(d).ok_or("couldn't find that device")?
    } else {
        host.default_input_device()
            .ok_or("no default input device")?
    };
    let default_config = device.default_input_config()?;
    let sample_format = default_config.sample_format();
    let mut config = default_config.config();

    if let Some(sr) = event.sr {
        config.sample_rate = cpal::SampleRate(sr);
    }
    if let Some(size) = event.buffer {
        config.buffer_size = cpal::BufferSize::Fixed(size);
    }
    if let Some(channels) = event.channels {
        config.channels = channels;
    }

    let c = config.channels as usize;
    let (s1, r1) = bounded(4096 * c);
    let (s2, r2) = bounded(4096 * c);

    let s = match sample_format {
        cpal::SampleFormat::F32 => run_in::<f32>(&device, &config, s1, s2),
        cpal::SampleFormat::I16 => run_in::<i16>(&device, &config, s1, s2),
        cpal::SampleFormat::U16 => run_in::<u16>(&device, &config, s1, s2),
        format => return Err(format!("unsupported sample format: {format}").into()),
    };
    if s.is_some() {
        stream.0 = s;
        audio_input_receiver1.0 = r1;
        audio_input_receiver2.0 = r2;
        in_stream_config.0 = Some(config);
        Ok(())
    } else {
        Err(format!("couldn't start stream with given settings\n{event:?}").into())
    }
}

fn run_out<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    slot: SlotBackend,
) -> Option<Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let mut slot = BlockRateAdapter::new(Box::new(slot));
    let channels = config.channels as usize;
    let mut out = vec![0.; channels];

    let err_fn = |err| eprintln!("an error occurred on stream: {err}");
    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            for frame in data.chunks_mut(channels) {
                slot.tick(&[], &mut out);
                for i in 0..channels {
                    let tmp = if out[i].is_normal() {
                        out[i].clamp(-1., 1.)
                    } else {
                        0.
                    };
                    frame[i] = T::from_sample(tmp);
                }
            }
        },
        err_fn,
        None,
    );
    if let Ok(stream) = stream {
        if let Ok(()) = stream.play() {
            return Some(stream);
        }
    }
    None
}

fn run_in<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    s1: Sender<(usize, f32)>,
    s2: Sender<(usize, f32)>,
) -> Option<Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = config.channels as usize;
    let err_fn = |err| eprintln!("an error occurred on stream: {err}");
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _| {
            for frame in data.chunks(channels) {
                for (channel, sample) in frame.iter().enumerate() {
                    let _ = s1.try_send((channel, sample.to_sample::<f32>()));
                    let _ = s2.try_send((channel, sample.to_sample::<f32>()));
                }
            }
        },
        err_fn,
        None,
    );
    if let Ok(stream) = stream {
        if let Ok(()) = stream.play() {
            return Some(stream);
        }
    }
    None
}

/// get a list of hosts and their input devices
pub fn list_in_devices() -> String {
    let mut s = String::new();
    let hosts = cpal::platform::ALL_HOSTS;
    s.push_str("input devices:\n");
    for (i, host) in hosts.iter().enumerate() {
        s.push_str(&format!("{i}: {host:?}:\n"));
        if let Ok(devices) = cpal::platform::host_from_id(*host).unwrap().input_devices() {
            for (j, device) in devices.enumerate() {
                s.push_str(&format!("    {}: {:?}\n", j, device.name()));
            }
        }
    }
    s
}

/// get a list of hosts and their output devices
pub fn list_out_devices() -> String {
    let mut s = String::new();
    let hosts = cpal::platform::ALL_HOSTS;
    s.push_str("output devices:\n");
    for (i, host) in hosts.iter().enumerate() {
        s.push_str(&format!("{i}: {host:?}:\n"));
        if let Ok(devices) = cpal::platform::host_from_id(*host)
            .unwrap()
            .output_devices()
        {
            for (j, device) in devices.enumerate() {
                s.push_str(&format!("    {}: {:?}\n", j, device.name()));
            }
        }
    }
    s
}
