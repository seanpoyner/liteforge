package com.liteforge;

import java.io.File;
import java.io.IOException;
import java.io.InputStream;
import java.nio.file.Files;
import java.nio.file.StandardCopyOption;

/**
 * Loads the {@code liteforge_java} native library. Tried in order:
 * <ol>
 *   <li>{@code System.loadLibrary} (requires {@code java.library.path})</li>
 *   <li>Extraction from the JAR at {@code /native/{os}-{arch}/}</li>
 * </ol>
 * All classes that use native methods should call {@link #ensureLoaded()} in a
 * static initializer.
 */
final class NativeLibrary {
    private static final String LIBRARY_NAME = "liteforge_java";
    private static volatile boolean loaded = false;

    private NativeLibrary() {}

    static synchronized void ensureLoaded() {
        if (loaded) {
            return;
        }
        try {
            System.loadLibrary(LIBRARY_NAME);
            loaded = true;
        } catch (UnsatisfiedLinkError e) {
            try {
                loadFromJar();
                loaded = true;
            } catch (IOException | UnsatisfiedLinkError ex) {
                throw new RuntimeException("Failed to load native library: " + LIBRARY_NAME, ex);
            }
        }
    }

    private static void loadFromJar() throws IOException {
        String osName = System.getProperty("os.name").toLowerCase();
        String osArch = System.getProperty("os.arch").toLowerCase();

        String libName;
        String libExtension;
        if (osName.contains("win")) {
            libExtension = ".dll";
            libName = LIBRARY_NAME;
        } else if (osName.contains("mac") || osName.contains("darwin")) {
            libExtension = ".dylib";
            libName = "lib" + LIBRARY_NAME;
        } else {
            libExtension = ".so";
            libName = "lib" + LIBRARY_NAME;
        }

        String arch;
        if (osArch.contains("amd64") || osArch.contains("x86_64")) {
            arch = "x64";
        } else if (osArch.contains("aarch64") || osArch.contains("arm64")) {
            arch = "arm64";
        } else {
            arch = osArch;
        }

        String resourcePath = "/native/" + osName + "-" + arch + "/" + libName + libExtension;
        try (InputStream is = NativeLibrary.class.getResourceAsStream(resourcePath)) {
            if (is == null) {
                throw new UnsatisfiedLinkError("Native library not found in JAR: " + resourcePath);
            }
            File tempFile = File.createTempFile(libName, libExtension);
            tempFile.deleteOnExit();
            Files.copy(is, tempFile.toPath(), StandardCopyOption.REPLACE_EXISTING);
            System.load(tempFile.getAbsolutePath());
        }
    }
}
