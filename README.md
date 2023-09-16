# goalnet

A payload and injector that enables easy .NET Core DLL injection.

## Motivation

A while ago, I was writing some injected C# code for the game FINAL FANTASY XIV. I ended up writing an injector and
loader using [dll-syringe][dll-syringe] and [netcorehost][netcorehost], supporting the ability to unload itself. Later
on, for another game, I wanted to reuse some its features and code, so I ended up writing it again a second time.
...then it came up a third time, and I questioned what I was doing with my life.

goalnet enables you to use C# in another process without having to think about the setup. Add a static method &
delegate, write a config file, and then start the injector. No extra thought required.

## Example

Let's use the `examples/hello-goalnet` folder as an example. This project loads into Notepad, shows a message box, and
unloads itself.

To demonstrate it in action, start Notepad, then build everything. goalnet-injector takes one argument, which is the
path to the config file.

```shell
$ cargo build --release
$ dotnet build --configuration Release ./examples/hello-goalnet/HelloGoalnet
$ cargo run --release --project goalnet-injector -- ./examples/hello-goalnet/config.toml
```

## Project setup

Every project that uses goalnet requires a config file, along with some setup in the assembly itself. See the above
example for all values in the config file.

In the project's `.csproj`, `GenerateRuntimeConfigurationFiles` must be set to `true`. This is required for goalnet, as
it creates a `.runtimeconfig.json` (used to determine what .NET version to use).

The assembly must have a class with an entrypoint method, along with a delegate matching the method signature. The
entrypoint method must be static, contain no arguments, and returns `void`.

When `unload` is set to true, the entrypoint method instead takes one argument, being an `IntPtr`/`nint` to the unload
function. This can be casted into a delegate, which can be called to signal unloading the assembly from inside itself.
This will only work as long as the injector process is still alive and communicating with the goalnet payload.

When `copy_build_artifacts` is set to true, the entire specified directory (along with the goalnet payload itself) is
copied to a temporary directory. This is wasteful, but allows you to bypass file locking issues, meaning you can cleanly
unload and reload the assembly without having to restart the target process.

[dll-syringe]: <https://github.com/OpenByteDev/dll-syringe>

[netcorehost]: <https://github.com/OpenByteDev/netcorehost>
