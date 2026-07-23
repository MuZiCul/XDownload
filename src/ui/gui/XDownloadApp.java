package ui.gui;

import com.formdev.flatlaf.FlatLightLaf;
import ui.gui.panels.*;
import ui.gui.workers.*;
import util.AppHome;
import util.Version;

import javax.swing.*;
import java.awt.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;

/**
 * GUI 入口：初始化 FlatLaf、日志重定向、启动主窗口
 */
public class XDownloadApp {

    public static void launch() {
        // 控制台输出重定向到日志文件（GUI 模式无控制台窗口）
        redirectLogs();

        // 设置 FlatLaf 现代皮肤
        try {
            UIManager.setLookAndFeel(new FlatLightLaf());
        } catch (Exception e) {
            try {
                UIManager.setLookAndFeel(UIManager.getSystemLookAndFeelClassName());
            } catch (Exception ignored) {}
        }

        // 启动主窗口
        SwingUtilities.invokeLater(() -> {
            MainFrame frame = new MainFrame();
            frame.setVisible(true);
        });
    }

    private static void redirectLogs() {
        try {
            Path logFile = AppHome.CONFIG.resolve("xdownload.log");
            Files.createDirectories(logFile.getParent());
            PrintStream log = new PrintStream(
                    new BufferedOutputStream(new FileOutputStream(logFile.toFile(), true)),
                    true, StandardCharsets.UTF_8);
            System.setOut(log);
            System.setErr(log);
        } catch (Exception ignored) {}
    }
}
