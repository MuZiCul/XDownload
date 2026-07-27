package ui.gui;

import ui.gui.workers.*;
import util.*;

import javax.swing.*;
import java.awt.*;
import java.awt.event.WindowAdapter;
import java.awt.event.WindowEvent;
import java.util.ArrayList;
import java.util.List;

public class StartupWizard extends JDialog {

    private final MainFrame mainFrame;
    private final CardLayout cards = new CardLayout();
    private final JPanel cardPanel = new JPanel(cards);

    private JLabel ytDlpStatus, ffmpegStatus;
    private JButton downloadToolsBtn, next1Btn;
    private JLabel netStatus, proxyTestResult;
    private JTextField proxyHost;
    private JSpinner proxyPort;
    private JButton testProxyBtn, next2Btn;
    private JComboBox<String> browserCombo;
    private JLabel cookiesStatus;
    private JButton validateCookiesBtn, finishBtn;

    /** 正在运行的 SwingWorker 列表，关闭弹窗时全部取消 */
    private final List<SwingWorker<?, ?>> runningWorkers = new ArrayList<>();

    public StartupWizard(MainFrame mainFrame) {
        super(mainFrame, I18n.get("wizard.title"), true);
        this.mainFrame = mainFrame;
        setSize(550, 440);
        setLocationRelativeTo(mainFrame);

        // 关闭弹窗时取消所有后台任务
        setDefaultCloseOperation(DISPOSE_ON_CLOSE);
        addWindowListener(new WindowAdapter() {
            @Override
            public void windowClosed(WindowEvent e) {
                cancelAllWorkers();
            }
        });

        buildCard1();
        buildCard2();
        buildCard3();

        add(cardPanel);
        cards.show(cardPanel, "env");
    }

    /** 取消所有正在执行的 SwingWorker */
    private void cancelAllWorkers() {
        synchronized (runningWorkers) {
            for (SwingWorker<?, ?> w : runningWorkers) {
                w.cancel(true);
            }
            runningWorkers.clear();
        }
        // 强制终止可能正在运行的 yt-dlp 进程
        mainFrame.downloader.cancel();
    }

    /** 注册 worker 以便关闭时取消 */
    private void registerWorker(SwingWorker<?, ?> worker) {
        synchronized (runningWorkers) {
            runningWorkers.add(worker);
        }
    }

    /** worker 完成后从列表移除 */
    private void unregisterWorker(SwingWorker<?, ?> worker) {
        synchronized (runningWorkers) {
            runningWorkers.remove(worker);
        }
    }

    // ==================== Card 1: 环境/组件 ====================

    private void buildCard1() {
        JPanel p = new JPanel(new BorderLayout(10, 10));
        p.setBorder(BorderFactory.createEmptyBorder(15, 15, 15, 15));
        p.add(new JLabel("<html><h2>" + I18n.get("wizard.env") + " (1/3)</h2></html>"), BorderLayout.NORTH);

        JPanel info = new JPanel();
        info.setLayout(new BoxLayout(info, BoxLayout.Y_AXIS));
        ytDlpStatus = new JLabel(I18n.get("wizard.ytdlp.checking"));
        ffmpegStatus = new JLabel(I18n.get("wizard.ffmpeg.checking"));
        info.add(ytDlpStatus);
        info.add(Box.createVerticalStrut(5));
        info.add(ffmpegStatus);
        p.add(info, BorderLayout.CENTER);

        JPanel btns = new JPanel(new FlowLayout(FlowLayout.RIGHT));
        downloadToolsBtn = new JButton(I18n.get("wizard.download.tools"));
        downloadToolsBtn.addActionListener(e -> downloadMissing());
        next1Btn = new JButton(I18n.get("wizard.next"));
        next1Btn.addActionListener(e -> cards.show(cardPanel, "proxy"));
        JButton skip1 = new JButton(I18n.get("wizard.skip"));
        skip1.addActionListener(e -> cards.show(cardPanel, "cookies"));
        btns.add(downloadToolsBtn);
        btns.add(skip1);
        btns.add(next1Btn);
        p.add(btns, BorderLayout.SOUTH);

        cardPanel.add(p, "env");
        checkTools();
    }

    private void checkTools() {
        // 快速文件检查（不启动进程）
        boolean ytOk = java.nio.file.Files.exists(Bootstrap.BIN_DIR.resolve("yt-dlp.exe"))
                || ProcessHelper.isYtDlpAvailable();
        ytDlpStatus.setText(I18n.get(ytOk ? "wizard.ytdlp.ok" : "wizard.ytdlp.no"));

        boolean ffOk = java.nio.file.Files.exists(Bootstrap.BIN_DIR.resolve("ffmpeg.exe"))
                || ProcessHelper.isFfmpegAvailable();
        ffmpegStatus.setText(I18n.get(ffOk ? "wizard.ffmpeg.ok" : "wizard.ffmpeg.no"));
    }

    /**
     * 下载缺失组件。
     * 下载前检测 GitHub 连通性：不可达则自动跳转代理页引导配置。
     */
    private void downloadMissing() {
        downloadToolsBtn.setEnabled(false);

        // ==== GitHub 连通性预检 ====
        if (!ProxyConfig.isEnabled()) {
            ytDlpStatus.setText("yt-dlp: checking GitHub...");
            boolean githubOk = NetworkDetect.isGithubAccessible();
            if (!githubOk) {
                ytDlpStatus.setText("yt-dlp: GitHub unreachable, configure proxy first");
                ffmpegStatus.setText("ffmpeg: waiting for proxy...");
                // 提示并跳转到代理卡片
                JOptionPane.showMessageDialog(this,
                        "Cannot reach GitHub.\nPlease configure a proxy on the next page.",
                        "Network Required", JOptionPane.WARNING_MESSAGE);
                cards.show(cardPanel, "proxy");
                downloadToolsBtn.setEnabled(true);
                return;
            }
        }

        // 先下载 yt-dlp，再下载 ffmpeg
        if (!ProcessHelper.isYtDlpAvailable()) {
            ytDlpStatus.setText("yt-dlp: downloading (~15MB)...");
            final BootstrapWorker[] ytRef = new BootstrapWorker[1];
            ytRef[0] = new BootstrapWorker(true, result -> {
                SwingUtilities.invokeLater(() -> {
                    ytDlpStatus.setText("yt-dlp: " + (result != null && !result.startsWith("failed")
                            ? I18n.get("status.ok") : "FAILED"));
                    unregisterWorker(ytRef[0]);
                    checkFFmpeg();
                });
            });
            registerWorker(ytRef[0]);
            ytRef[0].execute();
        } else {
            checkFFmpeg();
        }
    }

    private void checkFFmpeg() {
        if (!ProcessHelper.isFfmpegAvailable()) {
            ffmpegStatus.setText("ffmpeg: downloading (~80MB)...");
            final BootstrapWorker[] ffRef = new BootstrapWorker[1];
            ffRef[0] = new BootstrapWorker(false, result -> {
                SwingUtilities.invokeLater(() -> {
                    ffmpegStatus.setText("ffmpeg: " + (result != null && !result.startsWith("failed")
                            ? I18n.get("status.ok") : "skipped"));
                    unregisterWorker(ffRef[0]);
                    downloadToolsBtn.setEnabled(true);
                });
            });
            registerWorker(ffRef[0]);
            ffRef[0].execute();
        } else {
            downloadToolsBtn.setEnabled(true);
        }
    }

    // ==================== Card 2: 代理 ====================

    private void buildCard2() {
        JPanel p = new JPanel(new BorderLayout(10, 10));
        p.setBorder(BorderFactory.createEmptyBorder(15, 15, 15, 15));
        p.add(new JLabel("<html><h2>" + I18n.get("wizard.proxy.title") + " (2/3)</h2></html>"), BorderLayout.NORTH);

        JPanel center = new JPanel();
        center.setLayout(new BoxLayout(center, BoxLayout.Y_AXIS));

        // 预填 Windows 系统代理（如果有）
        String defaultHost = "127.0.0.1";
        int defaultPort = 7890;
        boolean hasSysProxy = ProxyConfig.detectSystemProxy();
        if (hasSysProxy) {
            defaultHost = ProxyConfig.getProxyHost();
            defaultPort = ProxyConfig.getProxyPort();
            // 注意：detectSystemProxy 会设置 enabled=true，但 wizard 中用户可能改，先保留检测结果
        }

        netStatus = new JLabel(I18n.get("proxy.detecting"));
        center.add(netStatus);

        JPanel input = new JPanel(new FlowLayout(FlowLayout.LEFT));
        input.add(new JLabel(I18n.get("proxy.host")));
        proxyHost = new JTextField(defaultHost, 12);
        input.add(proxyHost);
        input.add(new JLabel(I18n.get("proxy.port")));
        proxyPort = new JSpinner(new SpinnerNumberModel(defaultPort, 1, 65535, 1));
        input.add(proxyPort);

        // 系统代理提示
        if (hasSysProxy) {
            JLabel sysHint = new JLabel("已检测到Windows系统代理");
            sysHint.setForeground(new Color(0, 128, 0));
            sysHint.setFont(sysHint.getFont().deriveFont(11f));
            center.add(sysHint);
        }

        center.add(input);

        JPanel btns2 = new JPanel(new FlowLayout(FlowLayout.LEFT));
        testProxyBtn = new JButton(I18n.get("proxy.test"));
        testProxyBtn.addActionListener(e -> testProxy());
        btns2.add(testProxyBtn);
        proxyTestResult = new JLabel(" ");
        btns2.add(proxyTestResult);
        center.add(btns2);
        p.add(center, BorderLayout.CENTER);

        JPanel bottom = new JPanel(new FlowLayout(FlowLayout.RIGHT));
        JButton back2 = new JButton(I18n.get("wizard.back"));
        back2.addActionListener(e -> cards.show(cardPanel, "env"));
        next2Btn = new JButton(I18n.get("wizard.next"));
        next2Btn.addActionListener(e -> cards.show(cardPanel, "cookies"));
        JButton skip2 = new JButton(I18n.get("wizard.skip"));
        skip2.addActionListener(e -> cards.show(cardPanel, "cookies"));
        bottom.add(back2); bottom.add(next2Btn); bottom.add(skip2);
        p.add(bottom, BorderLayout.SOUTH);

        cardPanel.add(p, "proxy");

        // 后台异步检测网络环境（可取消）
        EnvironmentCheckWorker envWorker = new EnvironmentCheckWorker(overseas -> {
            SwingUtilities.invokeLater(() -> netStatus.setText(I18n.get(overseas ? "proxy.overseas" : "proxy.domestic")));
        });
        registerWorker(envWorker);
        envWorker.execute();
    }

    private void testProxy() {
        String host = proxyHost.getText().trim();
        int port = (Integer) proxyPort.getValue();
        proxyTestResult.setText(I18n.get("proxy.testing"));
        final ProxyTestWorker[] ptRef = new ProxyTestWorker[1];
        ptRef[0] = new ProxyTestWorker(host, port, result -> {
            SwingUtilities.invokeLater(() -> {
                unregisterWorker(ptRef[0]);
                if (result != null && result.success) {
                    proxyTestResult.setText("[+] " + I18n.get("proxy.ok") + result.elapsedMs + "ms");
                    ProxyConfig.setProxy(host, port);
                    ConfigManager.saveProxy(host, port);
                    mainFrame.refreshStatusBar();
                } else {
                    proxyTestResult.setText("[-] " + I18n.get("proxy.failed"));
                }
            });
        });
        registerWorker(ptRef[0]);
        ptRef[0].execute();
    }

    // ==================== Card 3: Cookies ====================

    private void buildCard3() {
        JPanel p = new JPanel(new BorderLayout(10, 10));
        p.setBorder(BorderFactory.createEmptyBorder(15, 15, 15, 15));
        p.add(new JLabel("<html><h2>" + I18n.get("wizard.cookies.title") + " (3/3)</h2></html>"), BorderLayout.NORTH);

        JPanel center = new JPanel();
        center.setLayout(new BoxLayout(center, BoxLayout.Y_AXIS));

        JPanel selPanel = new JPanel(new FlowLayout(FlowLayout.LEFT));
        selPanel.add(new JLabel(I18n.get("cookies.browser")));
        browserCombo = new JComboBox<>(new String[]{"chrome", "firefox", "edge", "brave", "opera"});
        selPanel.add(browserCombo);
        center.add(selPanel);

        JPanel valPanel = new JPanel(new FlowLayout(FlowLayout.LEFT));
        validateCookiesBtn = new JButton(I18n.get("cookies.validate"));
        validateCookiesBtn.addActionListener(e -> validateCookies());
        cookiesStatus = new JLabel(" ");
        valPanel.add(validateCookiesBtn);
        valPanel.add(cookiesStatus);
        center.add(valPanel);
        p.add(center, BorderLayout.CENTER);

        JPanel bottom = new JPanel(new FlowLayout(FlowLayout.RIGHT));
        JButton back3 = new JButton(I18n.get("wizard.back"));
        back3.addActionListener(e -> cards.show(cardPanel, "proxy"));
        finishBtn = new JButton(I18n.get("wizard.finish"));
        finishBtn.addActionListener(e -> saveAndClose());
        bottom.add(back3); bottom.add(finishBtn);
        p.add(bottom, BorderLayout.SOUTH);

        cardPanel.add(p, "cookies");

        // 后台扫描 cookies（可取消）
        scanCookiesAsync();
    }

    /** 后台异步扫描浏览器 cookies */
    private void scanCookiesAsync() {
        cookiesStatus.setText(I18n.get("cookies.scanning"));
        SwingWorker<String, Void> scanWorker = new SwingWorker<String, Void>() {
            @Override
            protected String doInBackground() {
                String[] browsers = {"chrome", "firefox", "edge", "brave", "opera"};
                for (String b : browsers) {
                    if (isCancelled()) return null;
                    ProcessHelper.CookiesValidationResult r =
                            ProcessHelper.validateCookiesFromBrowser(b);
                    if (r.success && r.cookieCount > 0) {
                        return b + "|" + r.cookieCount;
                    }
                }
                return null;
            }
            @Override
            protected void done() {
                unregisterWorker(this);
                try {
                    String result = get();
                    if (result != null) {
                        String[] parts = result.split("\\|");
                        browserCombo.setSelectedItem(parts[0]);
                        cookiesStatus.setText("[+] " + parts[0] + ": " + parts[1] + " cookies");
                    } else {
                        cookiesStatus.setText("[!] " + I18n.get("cookies.none"));
                    }
                } catch (Exception ignored) {}
            }
        };
        registerWorker(scanWorker);
        scanWorker.execute();
    }

    private void validateCookies() {
        String browser = (String) browserCombo.getSelectedItem();
        if (browser == null) return;
        cookiesStatus.setText(I18n.get("cookies.validating"));
        final CookiesValidationWorker[] cvRef = new CookiesValidationWorker[1];
        cvRef[0] = new CookiesValidationWorker(browser, result -> {
            SwingUtilities.invokeLater(() -> {
                unregisterWorker(cvRef[0]);
                cookiesStatus.setText(result != null ? result.message : "[-] Failed");
            });
        });
        registerWorker(cvRef[0]);
        cvRef[0].execute();
    }

    private void saveAndClose() {
        // 先取消所有运行中的后台任务
        cancelAllWorkers();
        String browser = (String) browserCombo.getSelectedItem();
        if (browser != null) {
            mainFrame.downloader.setCookiesFromBrowser(browser);
            ConfigManager.saveCookies(browser, null);
        }
        mainFrame.applySavedConfig();
        mainFrame.refreshStatusBar();
        dispose();
    }
}
