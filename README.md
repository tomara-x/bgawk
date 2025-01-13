[chicken](https://www.youtube.com/watch?v=pNiKW_f5ytM&t=19s)

this is a toy for playing with physics and sound

## building

- install rust: https://www.rust-lang.org/tools/install
- on linux:
    - install [bevy dependecies](https://github.com/bevyengine/bevy/blob/latest/docs/linux_dependencies.md)
    - install `libjack-dev` (`jack-devel` on void)
- clone
```
git clone https://github.com/tomara-x/bgawk.git
```
- build
```
cd bgawk
cargo run --release
```

> [!TIP]
> if you're building often, you might want to improve your compile times by dyanically linking bevy
> by appending `--features bevy/dynamic_linking` to your build or run commands
>
> [more info](https://bevyengine.org/learn/quick-start/getting-started/setup/#enable-fast-compiles-optional)

> [!TIP]
> you can configure bgawk via the command line or by placing settings at `$HOME/.config/bawk/config.toml`.
> for more information see [src/config.rs](./src/config.rs) or run:
> ```
> cargo run -- --help
> ```

## thanks

- avian https://github.com/Jondolf/avian
- bevy https://bevyengine.org
- fundsp https://github.com/SamiPerttu/fundsp
- bevy_egui https://github.com/vladbat00/bevy_egui
- egui https://github.com/emilk/egui
- syn https://github.com/dtolnay/syn
- cpal https://github.com/rustaudio/cpal
- bevy_pancam https://github.com/johanhelsing/bevy_pancam
- crossbeam_channel https://github.com/crossbeam-rs/crossbeam
- clap https://github.com/clap-rs/clap
- figment https://github.com/SergioBenitez/Figment
- xdg https://github.com/whitequark/rust-xdg
- serde https://github.com/serde-rs/serde

## random videos

https://www.youtube.com/playlist?list=PLW3qKRjtGsGaMXPz6lPiKr-BRqkUIv4Pl
