#include <iostream>
#include <string>
#include <regex>

#include <curl/curl.h>

#include <rapidjson/document.h>
#include <rapidjson/stringbuffer.h>
#include <rapidjson/writer.h>

#include <fstream>
#include <filesystem>

#ifdef _WIN32
#include <windows.h>
#include <shlwapi.h>
const std::string os = "windows";
#else
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
const std::string os = "linux";
#endif

std::string executeCommand(std::string cmd);
size_t WriteCallback(void* contents, size_t size, size_t nmemb, std::string* output);
std::string GetProgramDirectory();
std::string curlGet(std::string curlurl);
std::string replacePlaceholders(const std::string& input, const std::string& placeholder, const std::string& replacement);
std::string getJavaExecutablePath();
std::string escapeColon(const std::string& input);
std::string backslashesToForwardslashes(const std::string& input);

bool DirectoryExists(const std::string& directoryPath);
void CreateDirectoryIfNotExists(const std::string& directoryPath);
bool FileExists(const std::string& filePath);
void CreateFileIfNotExists(const std::string& filePath);

std::string readFileContents(const std::filesystem::path& filePath);
void saveJsonToFile(const rapidjson::Document& savedJson, std::string jsonSavePath);
void saveStringToFile(std::string savedString, std::string stringSavePath);

int main() {
    CURL* curl = curl_easy_init();
    rapidjson::Document document;
    std::string documentString = curlGet("https://launchermeta.mojang.com/mc/game/version_manifest.json");
    std::string programDirectory = GetProgramDirectory();
    if(documentString == "") {
        // read from manifest_cache.json
    } else {
        document.Parse(documentString.c_str());
        saveJsonToFile(document, programDirectory + "/manifest_cache.json");
    }

    if(os == "linux") {
        CreateFileIfNotExists(std::string(std::getenv("HOME")) + "/.config/alsoft.conf");
    }
    std::string latestVersion = document["latest"]["release"].GetString();

    //std::string latestVersion = "1.19.84 or something";
    if (!programDirectory.empty()) {
        CreateDirectoryIfNotExists(programDirectory + "/profiles");
        CreateDirectoryIfNotExists(programDirectory + "/assets/indexes");
        CreateDirectoryIfNotExists(programDirectory + "/assets/objects");
        CreateDirectoryIfNotExists(programDirectory + "/versions");
    } else {
        std::cerr << "Failed to retrieve program directory." << std::endl;
        return -1;
    }

    //executeCommand("ls"); to run a command. Returns std::string

    std::string version;
    std::cout << "Enter game version (latest is " << latestVersion << "): \n";
    //std::cin >> version;
    version = "1.19.4";
    std::string username;
    std::cout << "Enter your username: \n";
    //std::cin >> username;
    username = "humanitymanu";
    CreateDirectoryIfNotExists(programDirectory + "/versions/" + version);
    CreateDirectoryIfNotExists(programDirectory + "/versions/" + version + "/libraries");
    CreateDirectoryIfNotExists(programDirectory + "/profiles/" + version);

    rapidjson::Document versionJson;
    if(FileExists(programDirectory + "/versions/" + version + "/" + version + ".config")) {
        versionJson.Parse(readFileContents(programDirectory + "/versions/" + version + "/" + version + ".json").c_str());
        //executeCommand(". " + programDirectory + "/versions/" + version + "/" + version + ".config");
    } else {
        std::cout << "Downloading jsons and xml configs\n";
        CreateFileIfNotExists(programDirectory + "/versions/" +
                                version + "/" + version + ".config");


        int versionId;
        for(int i = 0; i < document["versions"].Size(); i++) {
            if(document["versions"][i]["id"].GetString() == version) {
                versionJson.Parse(curlGet(document["versions"][i]["url"].GetString()).c_str());
                versionId = i;
                break;
            }
        }
        rapidjson::Document assetJson;
        assetJson.Parse(curlGet(versionJson["assetIndex"]["url"].GetString()).c_str());
        saveJsonToFile(assetJson, programDirectory + "/assets/indexes/" + ((std::string) versionJson["assets"].GetString()) + ".json");
        saveJsonToFile(versionJson, programDirectory + "/versions/" + version + "/" + version + ".json");

        //logging
        std::string tempPath = programDirectory + "/versions/" + version + "/" + "logging-" + (std::string)versionJson["logging"]["client"]["file"]["id"].GetString();
        CreateFileIfNotExists(tempPath);
        saveStringToFile(curlGet(versionJson["logging"]["client"]["file"]["url"].GetString()), tempPath);

        saveStringToFile(curlGet(versionJson["downloads"]["client"]["url"].GetString()), programDirectory + "/versions/" + version + "/" + version + ".jar");

        // libraries
        std::string libBase = programDirectory + "/versions/" + version + "/libraries/";
        for(int i = 0; i < versionJson["libraries"].Size(); i++) {
            bool allowed = 1;
            std::string libName = libBase + versionJson["libraries"][i]["downloads"]["artifact"]["path"].GetString();
            std::filesystem::path filePath(libName);
            #ifdef _WIN32
            std::string libPath = filePath.parent_path().string();
            #else
            std::string libPath = filePath.parent_path();
            #endif
            std::string libUrl = versionJson["libraries"][i]["downloads"]["artifact"]["url"].GetString();
            //std::string libSha1 = versionJson["libraries"][i]["downloads"]["artifact"]["sha1"].GetString();
            // above is a planned feature for future versions. Not fully implemented
            std::cout << libName << "\n";
            if( (libName != (libBase + "/null")) && !(FileExists(libName)) ) {
                allowed = !(versionJson["libraries"][i].HasMember("rules"));
                if(!allowed) {
                    if(versionJson["libraries"][i]["rules"][0]["os"]["name"].GetString() == os) {
                        allowed = 1;
                    }
                }
                if(allowed) {
                    CreateDirectoryIfNotExists(libPath);
                    CreateFileIfNotExists(libName);
                    saveStringToFile(curlGet(libUrl), libName);
                    std::cout << "Downloading libraries: " << (i + 1) << " out of " << versionJson["libraries"].Size() << "\n";
                }
            }
        }
        if(!DirectoryExists(programDirectory + "/assets")) {
        CreateDirectoryIfNotExists(programDirectory + "/assets");
        const rapidjson::Value& objects = assetJson["objects"];
        int assetDownloadCounter = 1;
        for (rapidjson::Value::ConstMemberIterator it = objects.MemberBegin(); it != objects.MemberEnd(); ++it) {
            const std::string objHash = it->value["hash"].GetString();
            std::string id = objHash.substr(0, 2);
            std::string objFolder = programDirectory + "/assets/objects/" + id + "/";
            std::string objFile = objFolder + objHash;
            CreateDirectoryIfNotExists(objFolder);
            std::cout << "Downloading assets: " << assetDownloadCounter << " out of " << assetJson["objects"].MemberCount() << "\n";
            CreateFileIfNotExists(objFile);
            saveStringToFile(curlGet("https://resources.download.minecraft.net/" + id + "/" + objHash), objFile);
            assetDownloadCounter++;
        }
        }
    }

    std::string classPath;
    const rapidjson::Value& libraries = versionJson["libraries"];
    for (rapidjson::SizeType i = 0; i < libraries.Size(); ++i) {
        const std::string artifactPath = "libraries/" + ((std::string)libraries[i]["downloads"]["artifact"]["path"].GetString());
        const std::string fullPath = programDirectory + "/versions/" + version + "/" + artifactPath;
        //std::cout << fullPath << "\n";
        if (std::filesystem::exists(fullPath)) {
            classPath += escapeColon(backslashesToForwardslashes(fullPath));
            #ifdef _WIN32
            classPath += ";";
            #else
            classPath += ":";
            #endif
        }
    }
    classPath += escapeColon(backslashesToForwardslashes(programDirectory + "/versions/" + version + "/" + version + ".jar"));
    classPath = " -cp \"" + classPath + "\" ";

    std::string gameArgs = " ";
    if (versionJson.HasMember("minecraftArguments") && versionJson["minecraftArguments"].IsString()) {
        gameArgs = versionJson["minecraftArguments"].GetString();
        std::cout << "Game Arguments: " << gameArgs << "\n";
    } else {
        if (versionJson.HasMember("arguments") && versionJson["arguments"].HasMember("game") && versionJson["arguments"]["game"].IsArray()) {
            for(int i = 0; i < versionJson["arguments"]["game"].Size(); i++) {
                if(!(versionJson["arguments"]["game"][i].IsObject())) {
                    gameArgs += (std::string(versionJson["arguments"]["game"][i].GetString()) + " ");
                }
            }
        }
    }
    gameArgs = replacePlaceholders(gameArgs, "auth_player_name", username);
    gameArgs = replacePlaceholders(gameArgs, "version_name", version);
    gameArgs = replacePlaceholders(gameArgs, "game_directory", "\"" + backslashesToForwardslashes(programDirectory) + "/profiles/" + version + "\"");
    gameArgs = replacePlaceholders(gameArgs, "assets_root", "\"" + backslashesToForwardslashes(programDirectory) + "/assets/\"");
    gameArgs = replacePlaceholders(gameArgs, "auth_xuid", "0");
    gameArgs = replacePlaceholders(gameArgs, "auth_uuid", "0");
    gameArgs = replacePlaceholders(gameArgs, "auth_access_token", "0");
    gameArgs = replacePlaceholders(gameArgs, "clientid", "0");
    gameArgs = replacePlaceholders(gameArgs, "user_type", "legacy");
    gameArgs = replacePlaceholders(gameArgs, "version_type", "release");
    gameArgs = replacePlaceholders(gameArgs, "assets_index_name", versionJson["assetIndex"]["id"].GetString());

    std::string javaPath = getJavaExecutablePath();
    std::cout << "Java found: " << executeCommand("java -version") << " at " << javaPath << "\nDo you want to use this java, or enter another path manually?\n1 to use this, 0 to enter manually: ";
    bool javaManual;
    std::cin >> javaManual;
    std::string cmdJava;
    if(javaManual) {
        cmdJava = javaPath;
    } else {
        std::cout << "Enter path to java: ";
        std::cin >> cmdJava;
    }
    //cmdJava = "/home/mrmayman/Documents/Programs/jdk-17.0.6/bin/java";


    std::string cmdOpts = " -Xss1M -Djava.library.path=" + version + "-natives" +
                          " -Dminecraft.launcher.brand=minecraft-launcher" +
                          " -Dminecraft.launcher.version=2.1.1349 " +
                          replacePlaceholders(
                            std::string(versionJson["logging"]["client"]["argument"].GetString()),
                            "path",
                            "\"" + programDirectory + "/versions/" + version + "/logging-" +
                            versionJson["logging"]["client"]["file"]["id"].GetString() + "\""
                          ) +
                          " -Xmx2G -XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M";

    std::string finalCommand = cmdJava + cmdOpts + classPath + versionJson["mainClass"].GetString() + gameArgs;
    std::cout << finalCommand << "\n";
    #ifdef _WIN32

    // Windows-specific code
    STARTUPINFO si;
    PROCESS_INFORMATION pi;
    ZeroMemory(&si, sizeof(si));
    si.cb = sizeof(si);
    ZeroMemory(&pi, sizeof(pi));

    if (!CreateProcess(NULL, const_cast<char*>(finalCommand.c_str()), NULL, NULL, FALSE, 0, NULL, NULL, &si, &pi)) {
        std::cerr << "Failed to execute the command." << std::endl;
    } else {

    // Wait for the process to finish
    WaitForSingleObject(pi.hProcess, INFINITE);

    // Close process and thread handles
    CloseHandle(pi.hProcess);
    CloseHandle(pi.hThread);
    }

    #else
    std::cout << executeCommand(finalCommand);
    #endif
    std::cout << "shutting game down...\n";

    return 0;
}


std::string executeCommand(std::string cmd) {
    /*#ifdef _WIN32
    FILE* pipe = _popen(cmd.c_str(), "r");
    #else
    FILE* pipe = popen(cmd.c_str(), "r");
    #endif

    if (!pipe) { return "Error running command\n"; }

    char buffer[128];
    std::string result;

    while (!feof(pipe)) {
        if (fgets(buffer, 128, pipe) != nullptr) {
            result += buffer;
            std::cout << buffer;
        }
    }

    #ifdef _WIN32
    _pclose(pipe);
    #else
    pclose(pipe);
    #endif

    return result;*/
    std::string result;
    std::array<char, 128> buffer;

    #ifdef _WIN32
    FILE* pipe = _popen(cmd.c_str(), "r");
    #else
    FILE* pipe = popen(cmd.c_str(), "r");
    #endif

    if (!pipe) {
        return "Error running command\n";
    }

    while (fgets(buffer.data(), static_cast<int>(buffer.size()), pipe) != nullptr) {
        result += buffer.data();
        std::cout << buffer.data(); // Output the program's stdout in real-time
    }

    #ifdef _WIN32
    _pclose(pipe);
    #else
    pclose(pipe);
    #endif

    return result;
}


size_t WriteCallback(void* contents, size_t size, size_t nmemb, std::string* output) {
    size_t totalSize = size * nmemb;
    output->append(static_cast<char*>(contents), totalSize);
    return totalSize;
}


std::string GetProgramDirectory() {
#ifdef _WIN32
    char path[MAX_PATH];
    GetModuleFileName(NULL, path, MAX_PATH);
    PathRemoveFileSpec(path);
    return std::string(path);
#elif defined __linux__
    char path[PATH_MAX];
    ssize_t count = readlink("/proc/self/exe", path, PATH_MAX);
    if (count != -1) {
        path[count] = '\0';
        char* lastSlash = strrchr(path, '/');
        if (lastSlash) {
            *lastSlash = '\0';
            return std::string(path);
        }
    }
    return std::string();
#else
    // Unsupported platform
    return std::string();
#endif
}


bool DirectoryExists(const std::string& directoryPath) {
    std::filesystem::path directory(directoryPath);
    return std::filesystem::exists(directory) && std::filesystem::is_directory(directory);
}


void CreateDirectoryIfNotExists(const std::string& directoryPath) {
    if (!DirectoryExists(directoryPath)) {
        std::filesystem::path directory(directoryPath);
        if (!std::filesystem::create_directories(directory)) {
            std::cerr << "Failed to create directory: " << directoryPath << std::endl;
        }
    }
}


bool FileExists(const std::string& filePath) {
    std::filesystem::path file(filePath);
    return std::filesystem::exists(file) && std::filesystem::is_regular_file(file);
}


void CreateFileIfNotExists(const std::string& filePath) {
    if (FileExists(filePath)) {
        return;
    }
    std::ofstream file(filePath);
    if (file.is_open()) {
        file.close();
    } else {
        std::cerr << "Failed to create file." << std::endl;
    }
}

std::string curlGet(std::string curlurl) {
    CURL* curl = curl_easy_init();
    rapidjson::Document document;

    if (curl) {
        std::string response;
        // Set the URL for the request
        curl_easy_setopt(curl, CURLOPT_URL, curlurl.c_str());

        // Set the callback function to write the response into the string
        curl_easy_setopt(curl, CURLOPT_WRITEFUNCTION, WriteCallback);
        #ifdef _WIN32
        curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 0L);
        #endif
        curl_easy_setopt(curl, CURLOPT_WRITEDATA, &response);

        // Perform the request
        CURLcode res = curl_easy_perform(curl);
        curl_easy_cleanup(curl);
        // Check if the request was successful
        if (res == CURLE_OK) {
            return response;
        }
        std::cerr << "Failed to retrieve data: " << curl_easy_strerror(res) << "\n";
    } else {
        std::cerr << "Failed to initialize curl" << "\n";
    }
    return "";
}

void saveJsonToFile(const rapidjson::Document& savedJson, std::string jsonSavePath) {
    CreateFileIfNotExists(jsonSavePath);
    std::fstream file(jsonSavePath, std::ios::out);
    if (file.is_open()) {
        // Create a StringBuffer to serialize the JSON object
        rapidjson::StringBuffer buffer;
        rapidjson::Writer<rapidjson::StringBuffer> writer(buffer);
        // Serialize the JSON object to a string
        savedJson.Accept(writer);
        // Write the serialized string to the file
        file << buffer.GetString();
        // Close the file
        file.close();
    } else {
        std::cerr << "Failed to open the file." << std::endl;
    }
}

void saveStringToFile(std::string savedString, std::string stringSavePath) {
    std::fstream file(stringSavePath, std::ios::out);
    if (file.is_open()) {
        file << savedString;
        file.close();
    } else {
        std::cerr << "Failed to open file: " << stringSavePath << std::endl;
    }
}


std::string readFileContents(const std::filesystem::path& filePath) {
    std::ifstream file(filePath);
    if (!file.is_open()) {
        std::cerr << "Failed to open the file: " << filePath << std::endl;
        return "";
    }

    // Read the file contents into a string
    std::string fileContents((std::istreambuf_iterator<char>(file)),
                             std::istreambuf_iterator<char>());

    file.close();

    return fileContents;
}

std::string replacePlaceholders(const std::string& input, const std::string& placeholder, const std::string& replacement) {
    std::regex pattern("\\$\\{" + placeholder + "\\}");
    return std::regex_replace(input, pattern, replacement);
}

std::string getJavaExecutablePath() {
#ifdef _WIN32
    HKEY hKey;
    LONG result = RegOpenKeyEx(HKEY_LOCAL_MACHINE, "SOFTWARE\\JavaSoft\\Java Runtime Environment", 0, KEY_READ | KEY_WOW64_32KEY, &hKey);
    if (result == ERROR_SUCCESS) {
        char javaHome[MAX_PATH];
        DWORD size = sizeof(javaHome);
        result = RegQueryValueEx(hKey, "JavaHome", NULL, NULL, reinterpret_cast<LPBYTE>(javaHome), &size);
        RegCloseKey(hKey);
        if (result == ERROR_SUCCESS) {
            std::string javaExePath = javaHome;
            javaExePath += "\\bin\\java.exe";
            if (GetFileAttributesA(javaExePath.c_str()) != INVALID_FILE_ATTRIBUTES) {
                return javaExePath;
            }
        }
    }
#else
    return executeCommand("which java");
#endif

    return "";  // Return an empty string if Java is not found
}

std::string backslashesToForwardslashes(const std::string& input) {
    std::string escapedString;
    for (char ch : input) {
        if (ch == '\\') {
            escapedString += '/'; // Add an additional backslash
        } else {
            escapedString += ch;
        }
    }
    return escapedString;
}

std::string escapeColon(const std::string& input) {
    std::string escapedString;
    for (char ch : input) {
        /*if(ch == ':') {
            escapedString += "\\:";
        } else {*/
            escapedString += ch;
        //}
    }
    return escapedString;
}
