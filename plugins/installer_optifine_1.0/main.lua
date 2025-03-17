-- TODO: progress.send(Start)

local json = require("json")

local function split_three(str, sep)
    local p1, p2, p3 = str:match(("([^%s]+)%s([^%s]+)%s([^%s]+)"):format(sep, sep, sep))
    if not p1 or not p2 or not p3 then
        error("Library name cannot be split into 3 parts" .. str)
    end
    return p1, p2, p3
end

local function copy_details_json(inst, optifine_instance)
    local details = inst:join("details.json"):read()
    local details_json = json.decode(details)

    local new_details_path = optifine_instance:to_dot_mc_dir():join("versions"):join(details_json.id):join(details_json
        .id ..
        ".json")
    new_details_path:write(details)
end

local function install_libraries(optifine_inst)
    local function read_opti_json()
        local function find_fuzzy(inst, name)
            for entry in inst:read_dir() do
                local filename = entry:filename()
                if filename and filename:find(name) then
                    return entry
                end
            end
            error("Could not find required file in " .. tostring(inst) .. " matching: " .. name)
        end

        local opti_parent_dir = find_fuzzy(optifine_inst:to_dot_mc_dir():join("versions"), "Opti")
        local opti_json_path = find_fuzzy(opti_parent_dir, "json")
        return json.decode(opti_json_path:read())
    end

    local opti_json = read_opti_json()
    local libraries_dir = optifine_inst:to_dot_mc_dir():join("libraries")

    for i, library in ipairs(opti_json.libraries) do
        local name_check = "optifine"
        if not library.name:sub(1, #name_check) == name_check then
            -- Library is not an optifine builtin
            print(string.format("Downloading library (%d/%d): %s", i, #opti_json.libraries, library.name))

            local part1, part2, part3 = split_three(library.name, ".")
            local url_parent_path = part1:gsub(".", "/") .. "/" .. part2 .. part3;
            local url_final_part = string.format("%s/%s-%s.jar", url_parent_path, part2, part3)

            libraries_dir:join(url_parent_path):create_dir()
            local url = "https://libraries.minecraft.net/" .. url_final_part

            local jar_path = libraries_dir:join(url_final_part)
            if not jar_path:exists() then
                local file = qlDownload(url, false)
                jar_path:write(file)
            end
        end
    end
end

function Install(optifine_instance)
    qlLogInfo("Started installing OptiFine")
    local instance_dir = optifine_instance:to_instance_dir()
    copy_details_json(instance_dir, optifine_instance)

    local optifine_path = instance_dir:join("optifine")
    optifine_path:create_dir()

    local mc_path = tostring(optifine_instance:to_dot_mc_dir()):gsub("\\", "\\\\")
    local hook = java_OptifineInstaller:gsub("REPLACE_WITH_MC_PATH", mc_path)

    optifine_path:join("OptifineInstaller.java"):write(hook)
    local installer_path = optifine_path:join("OptiFine.jar")
    installer_path:write(qlPickFile("Select OptiFine installer jar file", { "jar" }, "Jar File"))

    qlLogInfo("Compiling OptifineInstaller.java")

    -- TODO: Java install progress
    -- TODO: progress.send(Compiling)
    qlJavaExec("javac", 21, nil, { "-cp", tostring(installer_path), "OptifineInstaller.java", "-d", "." },
        tostring(optifine_path))

    qlLogInfo("Running OptifineInstaller.java")
    -- TODO: progress.send(Running)
    qlJavaExec("java", 21, nil,
        { "-cp", tostring(installer_path) .. QL_CLASSPATH_SEPARATOR .. ".", "OptifineInstaller" },
        tostring(optifine_path))

    install_libraries(optifine_instance)

    local config_path = optifine_instance:to_instance_dir():join("config.json")
    local config = json.decode(config_path:read())
    config.mod_type = "OptiFine"
    local config_txt = json.encode(config)
    config_path:write(config_txt)
end

function Uninstall()
    print("TODO: Implement Uninstall")
end
