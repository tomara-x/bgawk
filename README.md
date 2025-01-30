[chicken](https://www.youtube.com/watch?v=pNiKW_f5ytM&t=19s)

this is a toy for playing with physics and sound. you can add rigid bodies with different properties, simulate gravity, add joints between them, link their properties to variables, and evaluate [lapis\*](https://github.com/tomara-x/lapis) code blocks on collision, and probably more :D (press f1 in the app for more detailed help)

[\*] lapis is a [FunDSP](https://github.com/SamiPerttu/fundsp) interpreter
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
> you can choose startup settings:
> - using command line arguments:
> ```
> cargo run -- --help
> ```
> - or by placing settings at `$HOME/.config/bgawk/config.toml` like so:
> ```pause = false
> fullscreen = false
> lapis_quiet = false
> lapis_keys = false
> gravity_x = 0
> gravity_y = 0
> attraction = 0.01
> scale_factor = 1
> win_width = 1280
> win_height = 720
> clear_color = "000000"
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
