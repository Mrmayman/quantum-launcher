use std::sync::{mpsc::Sender, Arc};

use mlua::{FromLua, Function, Lua, StdLib, UserData, UserDataMethods, Value, Variadic};

mod error;
pub use error::PluginError;
use ql_core::{err, get_java_binary, info, json::JavaVersion, pt, GenericProgress};

pub struct Plugin {
    lua: Lua,
    code: String,
}

impl Plugin {
    pub fn new(code: String) -> Result<Self, PluginError> {
        let lua = Lua::new();
        lua.load_std_libs(
            StdLib::BIT
                | StdLib::BUFFER
                | StdLib::COROUTINE
                | StdLib::MATH
                | StdLib::PACKAGE
                | StdLib::STRING
                | StdLib::TABLE
                | StdLib::UTF8
                | StdLib::VECTOR,
        )?;

        let globals = lua.globals();

        fn_logging(&globals, &lua)?;

        globals.set(
            "qlJavaExec",
            lua.create_async_function(|_, args| l_install_java(args))?,
        )?;

        Ok(Self { lua, code })
    }

    pub fn init(&self) -> Result<(), PluginError> {
        self.lua.load(&self.code).exec()?;
        Ok(())
    }

    pub fn call_fn(&self, name: &str) -> Result<(), PluginError> {
        let globals = self.lua.globals();
        let func: Function = globals.get(name)?;
        func.call::<()>(())?;
        Ok(())
    }
}

async fn l_install_java(args: (String, i32, Option<JavaProgress>)) -> Result<(), mlua::Error> {
    let version = match args.1 {
        8 => JavaVersion::Java8,
        16 => JavaVersion::Java16,
        17 => JavaVersion::Java17Gamma,
        170 => JavaVersion::Java17Beta,
        171 => JavaVersion::Java17GammaSnapshot,
        21 => JavaVersion::Java21,
        ver => {
            return Err(mlua::Error::ExternalError(Arc::new(StrErr(format!(
                "Could not determine valid java version: {ver}. Valid inputs"
            )))))
        }
    };
    let arc = args.2.map(|n| n.0.clone());
    get_java_binary(
        version,
        &args.0,
        if let Some(sender) = &arc {
            Some(&sender)
        } else {
            None
        },
    )
    .await
    .map_err(|err| mlua::Error::ExternalError(Arc::new(err)))?;
    Ok(())
}

#[derive(Debug)]
pub struct StrErr(String);

impl std::fmt::Display for StrErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for StrErr {}

#[derive(Clone)]
struct JavaProgress(Arc<Sender<GenericProgress>>);

impl FromLua for JavaProgress {
    fn from_lua(value: Value, _: &Lua) -> Result<Self, mlua::Error> {
        match value {
            Value::UserData(ud) => {
                let java_progress = ud.borrow::<Self>()?;
                Ok((*java_progress).clone())
            }
            _ => unreachable!(),
        }
    }
}

impl UserData for JavaProgress {
    fn add_methods<M: UserDataMethods<Self>>(_: &mut M) {
        // methods.add_method("magnitude", |_, vec, ()| {
        //     let mag_squared = vec.0 * vec.0 + vec.1 * vec.1;
        //     Ok(mag_squared.sqrt())
        // });

        // methods.add_meta_function(MetaMethod::Add, |_, (vec1, vec2): (Vec2, Vec2)| {
        //     Ok(Vec2(vec1.0 + vec2.0, vec1.1 + vec2.1))
        // });
    }
}

fn fn_logging(globals: &mlua::Table, lua: &Lua) -> Result<(), PluginError> {
    globals.set(
        "qlLogInfo",
        lua.create_function(|_, args: Variadic<Value>| {
            let mut msg = String::new();
            for arg in args {
                msg.push_str(&format!("{} ", arg.to_string()?));
            }
            info!("{msg}");
            Ok(())
        })?,
    )?;
    globals.set(
        "qlLogError",
        lua.create_function(|_, args: Variadic<Value>| {
            let mut msg = String::new();
            for arg in args {
                msg.push_str(&format!("{} ", arg.to_string()?));
            }
            err!("{msg}");
            Ok(())
        })?,
    )?;
    let fn_log_pt = lua.create_function(|_, args: Variadic<Value>| {
        let mut msg = String::new();
        for arg in args {
            msg.push_str(&format!("{} ", arg.to_string()?));
        }
        pt!("{msg}");
        Ok(())
    })?;
    globals.set("qlLogPt", fn_log_pt.clone())?;
    globals.set("print", fn_log_pt)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn sandbox_escape() {
        let plugin = Plugin::new(
            r#"
    local function test(name, func)
        local success, err = pcall(func)
        if success then
            error("[CRITICAL] " .. name .. " succeeded (should have failed)")
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

    -- 🛑 3. Attempt to require a networking module
    test("Network Access", function()
        local socket = require("socket")
        local client = socket.tcp()
        client:connect("example.com", 80)
        client:send("GET / HTTP/1.1\r\nHost: example.com\r\n\r\n")
        local response = client:receive("*a")
        client:close()
    end)

    -- 🛑 4. Attempt to require a dangerous module
    test("Require Unsafe Module", function()
        local posix = require("posix")  -- Only exists on Linux
    end)

    -- ✅ 5. Allowed Actions: Print & Stdout interaction
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
