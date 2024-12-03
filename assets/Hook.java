/*
QuantumLauncher
Copyright (C) 2024 Mrmayman & Contributors

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

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
