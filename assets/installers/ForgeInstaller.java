/*
 * This code is licensed under the Apache 2.0 license, unlike the rest of QuantumLauncher.
 * It is taken from https://github.com/alexivkin/minecraft-launcher/blob/master/ClientInstaller36.java
 * You can view the Apache 2.0 license in the APACHE_2_LICENSE file.
 */

import java.io.File;
import java.io.IOException;
import java.io.OutputStream;
import net.minecraftforge.installer.SimpleInstaller;
import net.minecraftforge.installer.actions.Actions;
import net.minecraftforge.installer.actions.ProgressCallback;
import net.minecraftforge.installer.json.InstallV1;
import net.minecraftforge.installer.json.Util;

// Renamed from ClientInstaller36
public class ForgeInstaller {

    public static void main(String[] args) throws IOException {
        SimpleInstaller.headless = true;
        System.setProperty("java.net.preferIPv4Stack", "true");
        ProgressCallback monitor = ProgressCallback.withOutputs(
            new OutputStream[] { System.out }
        );
        Actions action = Actions.CLIENT;
        try {
            InstallV1 install = Util.loadInstallProfile();
            File installer = new File(
                SimpleInstaller.class.getProtectionDomain()
                    .getCodeSource()
                    .getLocation()
                    .toURI()
            );
            // Line changed from:
            // if (!action.getAction(install, monitor).run(new File("."), a -> true,
            // installer)) {
            if (
                !action
                    .getAction(install, monitor)
                    .run(new File("."), installer)
            ) {
                System.out.println("Error");
                System.exit(1);
            }
            System.out.println(action.getSuccess());
        } catch (Throwable e) {
            e.printStackTrace();
            System.exit(1);
        }
        System.exit(0);
    }
}
