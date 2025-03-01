use ql_core::IoError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error(transparent)]
    Mlua(#[from] mlua::Error),
    #[error(transparent)]
    Io(#[from] IoError),
    #[error("json error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("plugin not found: name = {0}, version = {1:?}")]
    PluginNotFound(String, Option<String>),
}

mod plugin;
pub use plugin::Plugin;
mod resolve;
pub use resolve::install_plugins;
mod json;
mod passed_types;

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    pub fn sandbox_escape() {
        let plugin = Plugin::from_code(
            r#"
    local function test(name, func)
        local success, err = pcall(func)
        if success then
            error("[TEST FAIL] " .. name .. " succeeded (should have failed)")
        end
    end

    -- 1. Attempt to access the filesystem
    test("File Read", function()
        local file = io.open("/etc/passwd", "r")  -- Linux example, use C:\Windows\System32\config\SAM on Windows
        if file then
            file:close()
        end
    end)

    test("File Write", function()
        local file = io.open("test_sandbox.txt", "w")
        if file then
            file:close()
        end
    end)

    -- 2. Attempt to execute system commands
    test("System Command Execution", function()
        os.execute("ls")  -- Linux/Mac
        os.execute("dir")  -- Windows
    end)

    -- Attempt to require a networking module
    test("Network Access", function()
        local socket = require("socket")
        local client = socket.tcp()
        client:connect("example.com", 80)
        client:send("GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")
        local response = client:receive("*a")
        client:close()
    end)

    -- Attempt to require a dangerous module
    test("Require Unsafe Module", function()
        local posix = require("posix")  -- Only exists on Linux
    end)

    -- Allowed Actions: Print & Stdout interaction
    print("✅ This should be allowed")

    print("=== Sandbox Test Completed ===")
            "#
            .to_string(),
        )
        .unwrap();

        // Plugin should fail if any sandbox escape succeeded
        plugin.init().unwrap();
    }
}
