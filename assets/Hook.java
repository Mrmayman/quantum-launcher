import optifine.Installer;
import java.io.File;

// For hooking with the OptiFine installer.

public class Hook {
    public static void main(String[] args) {
        String directoryPath = "REPLACE_WITH_MC_PATH";
        File dirMc = new File(directoryPath);
        try {
            Installer.doInstall(dirMc);
        } catch (Exception e) {
            e.printStackTrace();
        }
    }
}
