package ui.gui.panels;

import ui.gui.MainFrame;
import util.ConfigManager;
import util.I18n;
import util.ProxyConfig;

import javax.swing.*;
import java.awt.*;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

/** 设置标签页 */
public class SettingsPanel extends JPanel {

    private final MainFrame mainFrame;
    private JTextField dirField;
    private JComboBox<String> langCombo;
    private ProxySettingsPanel proxyPanel;
    private CookiesSettingsPanel cookiesPanel;

    public SettingsPanel(MainFrame mainFrame) {
        this.mainFrame = mainFrame;
        setLayout(new BoxLayout(this, BoxLayout.Y_AXIS));
        setBorder(BorderFactory.createEmptyBorder(8, 8, 8, 8));

        add(buildDownloadDirPanel());
        add(Box.createVerticalStrut(8));
        proxyPanel = new ProxySettingsPanel(mainFrame);
        add(proxyPanel);
        add(Box.createVerticalStrut(8));
        cookiesPanel = new CookiesSettingsPanel(mainFrame);
        add(cookiesPanel);
        add(Box.createVerticalStrut(8));
        add(buildToolsPanel(mainFrame));
        add(Box.createVerticalStrut(8));
        add(buildLanguagePanel(mainFrame));
        add(Box.createVerticalStrut(8));
        add(buildBottomPanel());
        add(Box.createVerticalGlue());
    }

    // ==================== 各配置面板 ====================

    private JPanel buildDownloadDirPanel() {
        JPanel p = new JPanel(new FlowLayout(FlowLayout.LEFT, 5, 5));
        p.setBorder(BorderFactory.createTitledBorder(I18n.get("settings.dir")));

        String saved = util.ConfigManager.loadDownloadDir();
        dirField = new JTextField(saved != null ? saved : "downloads", 25);
        p.add(dirField);

        JButton browseBtn = new JButton(I18n.get("opt.browse"));
        browseBtn.addActionListener(e -> {
            JFileChooser fc = new JFileChooser();
            fc.setFileSelectionMode(JFileChooser.DIRECTORIES_ONLY);
            fc.setCurrentDirectory(new java.io.File(dirField.getText()));
            if (fc.showOpenDialog(this) == JFileChooser.APPROVE_OPTION) {
                dirField.setText(fc.getSelectedFile().getAbsolutePath());
            }
        });
        p.add(browseBtn);

        return p;
    }

    private JPanel buildLanguagePanel(MainFrame mainFrame) {
        JPanel p = new JPanel(new FlowLayout(FlowLayout.LEFT, 10, 5));
        p.setBorder(BorderFactory.createTitledBorder(I18n.get("lang.title")));

        langCombo = new JComboBox<>(new String[]{
                I18n.get("lang.zh"), I18n.get("lang.en")});
        langCombo.setSelectedIndex("en".equals(I18n.getLang()) ? 1 : 0);

        JButton applyBtn = new JButton(I18n.get("cookies.save"));
        applyBtn.addActionListener(e -> {
            String sel = (String) langCombo.getSelectedItem();
            String code = sel.equals(I18n.get("lang.en")) ? "en" : "zh";
            I18n.setLang(code);
            JOptionPane.showMessageDialog(mainFrame,
                    I18n.get("lang.restart"),
                    I18n.get("lang.title"), JOptionPane.INFORMATION_MESSAGE);
        });

        p.add(langCombo);
        p.add(applyBtn);
        return p;
    }

    private JPanel buildToolsPanel(MainFrame mainFrame) {
        JPanel p = new JPanel();
        p.setLayout(new BoxLayout(p, BoxLayout.Y_AXIS));
        p.setBorder(BorderFactory.createTitledBorder("Tools"));

        JPanel btnRow = new JPanel(new FlowLayout(FlowLayout.LEFT, 6, 2));
        boolean hasYt = util.ProcessHelper.isYtDlpAvailable();
        JButton ytBtn = new JButton(hasYt ? "yt-dlp: Latest" : "Download yt-dlp");
        ytBtn.setEnabled(!hasYt);
        ytBtn.addActionListener(e -> {
            ytBtn.setEnabled(false); ytBtn.setText("yt-dlp: ...");
            new ui.gui.workers.BootstrapWorker(true, result -> {
                SwingUtilities.invokeLater(() -> {
                    boolean ok = result != null && !result.startsWith("failed");
                    ytBtn.setText(ok ? "yt-dlp: Latest" : "Download yt-dlp");
                    ytBtn.setEnabled(!ok);
                    mainFrame.refreshStatusBar();
                });
            }).execute();
        });
        btnRow.add(ytBtn);

        boolean hasFf = util.ProcessHelper.isFfmpegAvailable();
        JButton ffBtn = new JButton(hasFf ? "ffmpeg: Latest" : "Download ffmpeg");
        ffBtn.setEnabled(!hasFf);
        ffBtn.addActionListener(e -> {
            ffBtn.setEnabled(false); ffBtn.setText("ffmpeg: ...");
            new ui.gui.workers.BootstrapWorker(false, result -> {
                SwingUtilities.invokeLater(() -> {
                    boolean ok = result != null && !result.startsWith("failed");
                    ffBtn.setText(ok ? "ffmpeg: Latest" : "Download ffmpeg");
                    ffBtn.setEnabled(!ok);
                    mainFrame.refreshStatusBar();
                });
            }).execute();
        });
        btnRow.add(ffBtn);

        JLabel hint = new JLabel(I18n.get("tools.hint"));
        hint.setFont(hint.getFont().deriveFont(11f));
        hint.setForeground(Color.GRAY);
        JPanel hintRow = new JPanel(new FlowLayout(FlowLayout.LEFT, 0, 0));
        hintRow.setOpaque(false);
        hintRow.add(hint);

        p.add(btnRow);
        p.add(hintRow);
        return p;
    }

    private JPanel buildBottomPanel() {
        JPanel p = new JPanel(new FlowLayout(FlowLayout.LEFT, 10, 5));

        JButton saveConfigBtn = new JButton("保存配置");
        saveConfigBtn.addActionListener(e -> saveAllConfig());

        JButton applyConfigBtn = new JButton("应用配置");
        applyConfigBtn.addActionListener(e -> applyAllConfig());

        JButton viewLogBtn = new JButton(I18n.get("settings.viewlog"));
        viewLogBtn.addActionListener(e -> {
            try {
                java.awt.Desktop.getDesktop().open(
                        util.AppHome.CONFIG.resolve("xdownload.log").toFile());
            } catch (Exception ex) {
                JOptionPane.showMessageDialog(this, "Cannot open log file: " + ex.getMessage());
            }
        });

        p.add(saveConfigBtn);
        p.add(applyConfigBtn);
        p.add(viewLogBtn);
        return p;
    }

    // ==================== 保存 / 应用配置 ====================

    /** 将所有面板的当前值保存到配置文件 */
    private void saveAllConfig() {
        Map<String, String> cfg = ConfigManager.load(); // 保留已有值，增量更新

        // 下载目录
        String dir = dirField.getText().trim();
        if (!dir.isEmpty()) cfg.put("downloadDir", dir);
        else cfg.remove("downloadDir");

        // 代理
        if (ProxyConfig.isEnabled()) {
            cfg.put("proxyHost", ProxyConfig.getProxyHost());
            cfg.put("proxyPort", String.valueOf(ProxyConfig.getProxyPort()));
        } else {
            cfg.remove("proxyHost");
            cfg.remove("proxyPort");
        }

        // Cookies
        String ckBrowser = mainFrame.downloader.getCookiesFromBrowser();
        String ckFile = mainFrame.downloader.getCookiesFile();
        if (ckBrowser != null && !ckBrowser.isEmpty()) {
            cfg.put("cookiesFromBrowser", ckBrowser);
            cfg.remove("cookiesFile");
        } else if (ckFile != null && !ckFile.isEmpty()) {
            cfg.put("cookiesFile", ckFile);
            cfg.remove("cookiesFromBrowser");
        }

        // 语言
        String sel = (String) langCombo.getSelectedItem();
        String code = sel != null && sel.equals(I18n.get("lang.en")) ? "en" : "zh";
        cfg.put("lang", code);

        ConfigManager.save(cfg);

        String configPath = ConfigManager.getConfigFile().toAbsolutePath().toString();
        JOptionPane.showMessageDialog(this,
                "配置已保存到:\n" + configPath,
                "保存配置", JOptionPane.INFORMATION_MESSAGE);
    }

    /** 从配置文件读取并应用到所有面板 */
    private void applyAllConfig() {
        // 弹窗让用户选择配置来源
        boolean hasDefault = Files.exists(ConfigManager.getConfigFile());
        String[] options = hasDefault
                ? new String[]{"应用默认配置", "选择配置文件位置", "取消"}
                : new String[]{"选择配置文件位置", "取消"};

        int choice = JOptionPane.showOptionDialog(this,
                hasDefault ? "请选择配置来源" : "默认配置文件不存在，请选择配置文件",
                "应用配置", JOptionPane.DEFAULT_OPTION, JOptionPane.QUESTION_MESSAGE,
                null, options, options[0]);

        Map<String, String> cfg = null;
        if (hasDefault && choice == 0) {
            cfg = ConfigManager.load();
        } else if ((hasDefault && choice == 1) || (!hasDefault && choice == 0)) {
            JFileChooser fc = new JFileChooser();
            fc.setFileFilter(new javax.swing.filechooser.FileNameExtensionFilter("JSON 配置文件", "json"));
            Path cfgDir = ConfigManager.getConfigFile().getParent();
            if (Files.exists(cfgDir)) fc.setCurrentDirectory(cfgDir.toFile());
            if (fc.showOpenDialog(this) == JFileChooser.APPROVE_OPTION) {
                cfg = loadConfigFile(fc.getSelectedFile().toPath());
                if (cfg == null) {
                    JOptionPane.showMessageDialog(this,
                            "无法读取配置文件，请检查文件格式。",
                            "应用配置", JOptionPane.ERROR_MESSAGE);
                    return;
                }
            } else {
                return;
            }
        } else {
            return;
        }

        applyConfigMap(cfg);
    }

    /** 核心：将配置 Map 应用到各面板 */
    private void applyConfigMap(Map<String, String> cfg) {
        int total = 0, applied = 0;
        StringBuilder missing = new StringBuilder();
        StringBuilder invalid = new StringBuilder();

        // 有效值白名单
        final java.util.Set<String> VALID_BROWSERS = java.util.Set.of("chrome", "firefox", "edge", "brave", "opera");
        final java.util.Set<String> VALID_LANGS = java.util.Set.of("zh", "en");

        // 1. 下载目录
        total++;
        String dir = cfg.get("downloadDir");
        if (dir != null && !dir.isEmpty()) {
            dirField.setText(dir);
            applied++;
        }

        // 2. 代理
        total++;
        String host = cfg.get("proxyHost");
        String portStr = cfg.get("proxyPort");
        if (host != null && !host.isEmpty() && portStr != null && !portStr.isEmpty()) {
            try {
                int port = Integer.parseInt(portStr);
                ProxyConfig.setProxy(host, port);
                applied++;
            } catch (NumberFormatException e) {
                missing.append("代理端口号无效; ");
            }
        } else {
            missing.append("代理; ");
        }

        // 3. Cookies（校验浏览器名白名单）
        total++;
        String ckBrowser = cfg.get("cookiesFromBrowser");
        String ckFile = cfg.get("cookiesFile");
        if (ckBrowser != null && !ckBrowser.isEmpty()) {
            if (VALID_BROWSERS.contains(ckBrowser)) {
                mainFrame.downloader.setCookiesFromBrowser(ckBrowser);
                applied++;
            } else {
                invalid.append("Cookies浏览器 \"" + ckBrowser + "\" 无效，已忽略; ");
            }
        } else if (ckFile != null && !ckFile.isEmpty()) {
            mainFrame.downloader.setCookiesFile(ckFile);
            applied++;
        } else {
            missing.append("Cookies; ");
        }

        // 4. 语言（校验 zh/en 白名单）
        total++;
        String lang = cfg.get("lang");
        if (lang != null && !lang.isEmpty()) {
            if (VALID_LANGS.contains(lang)) {
                langCombo.setSelectedItem("en".equals(lang) ? I18n.get("lang.en") : I18n.get("lang.zh"));
                applied++;
            } else {
                invalid.append("语言 \"" + lang + "\" 无效，已忽略; ");
            }
        }

        // 刷新 UI
        proxyPanel.reflectConfig();
        cookiesPanel.reflectCookies();
        mainFrame.downloadPanel.applyCookiesToDownloader();
        mainFrame.refreshStatusBar();

        // 弹出结果
        if (applied == total) {
            JOptionPane.showMessageDialog(this,
                    "已应用全部 " + applied + " 项配置",
                    "应用配置", JOptionPane.INFORMATION_MESSAGE);
        } else if (applied > 0) {
            StringBuilder msg = new StringBuilder();
            msg.append("已应用 ").append(applied).append("/").append(total).append(" 项。");
            if (missing.length() > 0) msg.append("\n未配置项: ").append(missing);
            if (invalid.length() > 0) msg.append("\n无效字段: ").append(invalid);
            JOptionPane.showMessageDialog(this, msg.toString(),
                    "应用配置（部分）", JOptionPane.WARNING_MESSAGE);
        } else {
            JOptionPane.showMessageDialog(this,
                    "配置文件中没有可用的设置项。",
                    "应用配置", JOptionPane.WARNING_MESSAGE);
        }
    }

    /** 从指定路径的 JSON 文件加载配置 Map */
    private Map<String, String> loadConfigFile(Path file) {
        try {
            String json = Files.readString(file);
            Map<String, String> map = new LinkedHashMap<>();
            int start = json.indexOf('{');
            int end = json.lastIndexOf('}');
            if (start < 0 || end < 0) return null;
            String body = json.substring(start + 1, end);
            Pattern p = Pattern.compile("\"([^\"]+)\"\\s*:\\s*\"([^\"]*)\"");
            Matcher m = p.matcher(body);
            while (m.find()) {
                map.put(m.group(1), m.group(2));
            }
            Pattern numP = Pattern.compile("\"([^\"]+)\"\\s*:\\s*(\\d+)");
            Matcher numM = numP.matcher(body);
            while (numM.find()) {
                map.putIfAbsent(numM.group(1), numM.group(2));
            }
            return map;
        } catch (IOException e) {
            return null;
        }
    }
}
