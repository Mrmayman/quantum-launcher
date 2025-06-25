This will mainly focus on what the
codebase is like for any potential contributors.

Btw, if you have any questions, feel free to ask me on [Discord](https://discord.gg/bWqRaSXar5)!

# Crate Structure
- `quantum_launcher` - The GUI frontend
- `ql_instances` - Instance management, updating and launching
- `ql_mod_manager` - Mod management and installation
- `ql_servers` - A self-hosted server management system (incomplete)
- `ql_packager` - Code related to packaging/importing instances
### Core components
- `ql_core` - Core utilities and shared code
- `ql_reqwest` - A shim (wrapper) around the [reqwest](https://github.com/seanmonstar/reqwest) library, that automatically deals with platform-specific features in the Cargo.toml.
### Specific-purpose "libraries"
- `ql_java_handler` - A library to auto-install and provide java runtimes

# UI Pattern
The architecture of the launcher is based on the
Model-View-Controller pattern (AKA the thing used in iced).

- The `Launcher` struct is the main controller of the application.
- `view()` renders the app's view based on the current state.
- `update()` processes messages and updates the state accordingly.
- The `state::State` enum determines which menu is currently open.

So it's a back-and-forth between `Message`s coming from interaction,
and code to deal with the messages in `update()`.

# Comments
I tend to be loose, for better or for worse,
when it comes to using comments.
Have something complicated-looking that could
be better explained? Add comments. Clippy bugging you
about not documenting something? Add doc comments.

**The only rule of thumb is: Do it well or don't do it**.

Half-baked useless comments are worse than no comments
(yes I'm guilty of this sometimes).

Heck, feel free to make it informal if that seems better.
(maybe add a `WTF: ` tag so people can search for it for fun).

# Helpers/Patterns
## 1) Logging

Use `info!()`, `pt!()`, `err!()` for launcher log messages. Here's an example

```txt
[info] Installing something
- Doing step 1...
- Doing step 2...
- Another point message
[error] 404 when downloading library at (https://example.com/downloads/something.jar), skipping...
```

Make sure to only log useful things, don't spew endless garbage.
As a rule of thumb, if a user's launcher breaks and they send it,
that message should be one that helps in troubleshooting/understanding what happened.

- Info is for... well, informational messages that tell
  what's going on as a big-picture.
- Pt (point) is for small informational messages for steps,
  small details and stuff, just extra info.
- Err is for errors. Note: Try to return errors in `Result<T, E>`
  if it can't be recovered from. `err!()` is for "warning" kind
  of errors that can be ignored/skipped safely.

There is no warn/warning macro because non-fatal errors are `err!()`
and fatal ones are returned.

## 2) IO
Try to make as much of the code dealing with filesystem or network,
async, when possible. This rule can occasionally be broken
but it's recommended to follow this.

Use `tokio::fs` for filesystem operations,
and use `ql_core::file_utils::download_file_to_*` functions for networking.

There are actually a lot of nice goodies in `ql_core::file_utils`,
feel free to read through it or check `cargo doc`.

It's a common pattern to import `ql_core::file_utils`
and manually call `file_utils::*`.

## 3) Errors
Try to return any fatal errors as `Result<T, E>`,
those that can't be ignored/bypassed. It's generally recommended
to make your custom error enum for a specific task or group of tasks.
For example, `ForgeInstallError`, `FabricInstallError`, `GameLaunchError`, and so on.

`Box<dyn Error>` is **frowned upon**, I recommend you instead just
convert the errors to `String`s for such use cases.
You can call `.strerr()` on any `Result`s
(using the `ql_core::IntoStringError` trait) to do that easily.

Use `thiserror` and `#[derive(Debug, thiserror::Error)]` for your
error types. All errors must implement `Debug`, `thiserror::Error` and `Display`.

Use `#[from]` and `#[error]` syntax of thiserror when needed.

```rust
use thiserror::Error;

const MY_ERR_PREFIX: &str = "while doing my thing\n:";

#[derive(Debug, Error)]
enum MyError {
    // Add context for third-party errors
    #[error("{MY_ERR_PREFIX}while extracting zip:\n{0}")]
    Zip(ZipError),

    // But not for QuantumLauncher-defined errors
    #[error("{MY_ERR_PREFIX}{0}")]
    Io(#[from] IoError),
    #[error("{MY_ERR_PREFIX}{0}")]
    Request(#[from] RequestError),

    #[error("{MY_ERR_PREFIX}no valid forge version found")]
    NoForgeVersionFound,
}
```

Any errors that can be potentially shown in the user interface,
should read out something like:

```txt
while doing my thing:
while installing forge:
while extracting installer:
Zip file contains invalid data!
```

Try to make it user-friendly, any average guy should be able to understand.
For very common errors like IO or network errors,
add some user-friendly points on what to do
and put the "ugly cruft" away at the bottom. You probably
won't encounter this situation much.

Capitalization is up to you, do what feels right!

## 4) Error Magic

There are a few extra methods for `Result<T, E>`
from various traits, that help in error handling:

### `.path(your_path)` (from `ql_core::IntoIoError` trait)
This converts the basic `std::io::Error`
into a nicer `ql_core::IoError`.

Call this method on `Result<T, std::io::Error>`.

For example:

```rust
tokio::fs::write(&path, &bytes).await.path(path)?;
```


### `.json(original_string)` (from `ql_core::IntoJsonError`)
For parsing json **strings** into structs.

Call this method on `serde_json`'s error's Result.

### `.json_to()` (from `ql_core::IntoJsonError`)
For converting **structs** into json strings.

Call this method on `serde_json`'s error's Result.

### `.strerr()` (from `ql_core::IntoStringError`)
For converting any error into `Result<T, String>`.
Useful for "dynamic" or "generic" errors.

This is also needed for any async functions called
by the GUI.

**More docs coming in the future...**
