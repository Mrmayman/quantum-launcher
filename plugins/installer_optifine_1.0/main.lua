-- Defined:
--  optifine_instance (SelectedInstance)
--  optifine_installer_bytes: string (bytes)

-- TODO: progress.send(Start)

local json = require("json")

qlLogInfo("Started installing OptiFine")
local instance_dir = optifine_instance:to_instance_dir()

local details = instance_dir:join("details.json"):read()
local details_json = json.decode(details)

local new_details_path = optifine_instance:to_dot_mc_dir():join("versions"):join(details_json.id):join(details_json.id ..
    ".json")
new_details_path:write(details)

local optifine_path = instance_dir:join("optifine")
optifine_path:create_dir()

local mc_path = tostring(optifine_instance:to_dot_mc_dir()):gsub("\\", "\\\\")
local hook = java_OptifineInstaller:gsub("REPLACE_WITH_MC_PATH", mc_path)

optifine_path:join("OptifineInstaller.java"):write(hook)
local installer_path = optifine_path:join("OptiFine.jar")
installer_path:write(optifine_installer_bytes)

qlLogInfo("Compiling OptifineInstaller.java")

-- TODO: Java install progress
-- TODO: progress.send(Compiling)
qlJavaExec("javac", 21, nil, { "-cp", tostring(installer_path), "OptifineInstaller.java", "-d", "." },
    tostring(optifine_path))

qlLogInfo("Running OptifineInstaller.java")
-- TODO: progress.send(Running)
qlJavaExec("java", 21, nil,
    { "-cp", tostring(installer_path) .. CLASSPATH_SEPARATOR .. ".", "OptifineInstaller" }, tostring(optifine_path))
