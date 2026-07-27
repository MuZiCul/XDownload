package ui.gui.panels;

import util.I18n;
import downloader.YtDlpDownloader;
import downloader.YtDlpDownloader.DownloadProgress;
import model.DownloadConfig;
import model.VideoInfo;
import ui.gui.MainFrame;
import ui.gui.workers.DownloadWorker;
import ui.gui.workers.FetchInfoWorker;

import javax.swing.*;
import java.awt.*;
import java.io.File;

/**
 * 下载标签页：URL 输入 + 视频信息 + 格式表 + 选项 + 进度条
 */
public class DownloadPanel extends JPanel {

    final MainFrame mainFrame;
    private final URLInputPanel urlInputPanel;
    private final VideoInfoPanel videoInfoPanel;
    private final FormatTablePanel formatTablePanel;
    public final DownloadOptionsPanel optionsPanel = new DownloadOptionsPanel(this);
    private final ProgressPanel progressPanel;

    private VideoInfo currentVideoInfo;
    private DownloadConfig currentConfig;
    private FetchInfoWorker fetchWorker;
    private DownloadWorker downloadWorker;

    public DownloadPanel(MainFrame mainFrame) {
        super(new BorderLayout(5, 5));
        this.mainFrame = mainFrame;
        setBorder(BorderFactory.createEmptyBorder(8, 8, 8, 8));

        // 顶部：URL 输入
        urlInputPanel = new URLInputPanel(this);

        // 中间：左右分栏
        videoInfoPanel = new VideoInfoPanel();
        formatTablePanel = new FormatTablePanel(this);

        JPanel leftPanel = new JPanel(new BorderLayout());
        leftPanel.add(videoInfoPanel, BorderLayout.NORTH);
        leftPanel.add(formatTablePanel, BorderLayout.CENTER);

        // 右侧面板：格式选择栏
        JPanel rightPanel = new JPanel(new BorderLayout());
        rightPanel.add(buildFormatBar(), BorderLayout.NORTH);

        JSplitPane splitPane = new JSplitPane(JSplitPane.HORIZONTAL_SPLIT, leftPanel, rightPanel);
        splitPane.setResizeWeight(0.6);

        // 底部：进度条
        progressPanel = new ProgressPanel();

        add(urlInputPanel, BorderLayout.NORTH);
        add(splitPane, BorderLayout.CENTER);
        add(progressPanel, BorderLayout.SOUTH);
    }

    /** 用户点击 Fetch Info */
    public void fetchVideoInfo(String url) {
        LogPanel.log("[INFO] Fetching: " + url);
        progressPanel.setStatus(I18n.get("prog.fetching"));
        progressPanel.setIndeterminate(true);
        fetchWorker = new FetchInfoWorker(mainFrame.downloader, url, this);
        fetchWorker.execute();
    }

    /** FetchInfoWorker 完成回调 */
    public void onVideoInfoReady(VideoInfo info) {
        this.currentVideoInfo = info;
        this.currentConfig = new DownloadConfig(info.getUrl());
        videoInfoPanel.setVideoInfo(info);
        formatTablePanel.setFormats(info.getFormats());
        progressPanel.setIndeterminate(false);
        progressPanel.setStatus("[+] " + I18n.get("prog.ready"));
    }

    /** FetchInfoWorker 失败回调 */
    public void onFetchError(String error) {
        progressPanel.setIndeterminate(false);
        progressPanel.setError("[-] " + error);
        JOptionPane.showMessageDialog(this, error, I18n.get("prog.error.title"), JOptionPane.ERROR_MESSAGE);
    }

    public DownloadConfig getCurrentConfig() { return currentConfig; }

    /** 格式选中 */
    public void onFormatSelected(String formatId) {
        if (currentConfig != null) {
            currentConfig.setFormatId(formatId);
        }
    }

    /** 开始下载 */
    public void startDownload(DownloadConfig config) {
        this.currentConfig = config;
        LogPanel.log("[INFO] Download: " + config.getUrl() + " | fmt=" + config.getFormatId() + " | dir=" + config.getOutputDir());
        progressPanel.reset();
        progressPanel.setStatus("[...] " + I18n.get("prog.downloading"));
        downloadWorker = new DownloadWorker(mainFrame.downloader, config, this);
        downloadWorker.execute();
    }

    /** 取消下载 */
    public void cancelDownload() {
        if (downloadWorker != null && !downloadWorker.isDone()) {
            downloadWorker.cancel(true);
            mainFrame.downloader.cancel();
            progressPanel.setStatus("[-] " + I18n.get("prog.cancelled"));

        }
    }

    /** 下载进度回调 */
    public void onDownloadProgress(DownloadProgress progress) {
        String eta = progress.eta != null && !progress.eta.isEmpty() && !"NA".equals(progress.eta) ? " ETA:" + progress.eta : "";
        setDownloadStatus(progress.percent + " | " + (progress.speed != null ? progress.speed : "?") + eta);
    }

    /** 下载完成 */
    public void onDownloadComplete(boolean success, String outputPath) {
                if (success) {
            progressPanel.setComplete();
            LogPanel.log("[OK] Saved to: " + outputPath);
            JOptionPane.showMessageDialog(this,
                    I18n.get("prog.complete.msg") + outputPath,
                    I18n.get("prog.done.title"), JOptionPane.INFORMATION_MESSAGE);
        } else {
            progressPanel.setError("[-] " + I18n.get("prog.failed"));
            LogPanel.log("[FAIL] Download error — check Log tab for yt-dlp output");
        }
    }

    private JPanel statusLine1, statusLine2, statusLine3, statusLine4;
    private JLabel downloadStatusLabel;

    /** 构建右侧下载面板：配置状态 + 格式选择 + 进度 */
    private JPanel buildFormatBar() {
        JPanel box = new JPanel(new BorderLayout(0, 4));
        box.setBorder(BorderFactory.createTitledBorder(I18n.get("opt.start")));

        // 上部：配置状态 + 分界线
        JPanel topPanel = new JPanel(new BorderLayout());
        topPanel.setBorder(BorderFactory.createEmptyBorder(6, 0, 0, 0));

        JPanel statusPanel = new JPanel();
        statusPanel.setBorder(BorderFactory.createEmptyBorder(6, 0, 6, 0));
        statusPanel.setLayout(new BoxLayout(statusPanel, BoxLayout.Y_AXIS));
        statusLine1 = new JPanel(new FlowLayout(FlowLayout.LEFT, 0, 0));
        statusLine2 = new JPanel(new FlowLayout(FlowLayout.LEFT, 0, 0));
        statusLine3 = new JPanel(new FlowLayout(FlowLayout.LEFT, 0, 0));
        statusLine4 = new JPanel(new FlowLayout(FlowLayout.LEFT, 0, 0));
        statusLine1.setAlignmentX(Component.LEFT_ALIGNMENT); statusLine2.setAlignmentX(Component.LEFT_ALIGNMENT);
        statusLine3.setAlignmentX(Component.LEFT_ALIGNMENT); statusLine4.setAlignmentX(Component.LEFT_ALIGNMENT);
        statusLine1.setOpaque(false); statusLine2.setOpaque(false);
        statusLine3.setOpaque(false); statusLine4.setOpaque(false);
        statusPanel.add(statusLine1);
        statusPanel.add(statusLine2);
        statusPanel.add(statusLine3);
        statusPanel.add(statusLine4);
        refreshStatusLines();
        topPanel.add(statusPanel, BorderLayout.NORTH);
        box.add(topPanel, BorderLayout.NORTH);

        // 中部：分隔线 + 格式选择 + 下载按钮
        JPanel midPanel = new JPanel();
        midPanel.setLayout(new BoxLayout(midPanel, BoxLayout.Y_AXIS));
        midPanel.add(new JSeparator());
        midPanel.add(Box.createVerticalStrut(4));

        JPanel row1 = new JPanel(new FlowLayout(FlowLayout.LEFT, 4, 2));
        formatTablePanel.bestRadio.addActionListener(e -> onFormatSelected("best"));
        formatTablePanel.bestVARadio.addActionListener(e -> onFormatSelected("bestvideo+bestaudio/best"));
        formatTablePanel.audioRadio.addActionListener(e -> onFormatSelected("bestaudio"));
        row1.add(formatTablePanel.bestRadio);
        row1.add(formatTablePanel.bestVARadio);
        row1.add(formatTablePanel.audioRadio);
        row1.setAlignmentX(Component.LEFT_ALIGNMENT);
        midPanel.add(row1);

        JPanel row2 = new JPanel(new FlowLayout(FlowLayout.LEFT, 6, 4));
        row2.setAlignmentX(Component.LEFT_ALIGNMENT);
        row2.add(new JLabel(I18n.get("opt.retries")));
        JSpinner retriesSpinner = new JSpinner(new SpinnerNumberModel(5, 0, 99, 1));
        retriesSpinner.setPreferredSize(new Dimension(55, 22));
        row2.add(retriesSpinner);

        JButton startBtn = new JButton(I18n.get("opt.download"));
        startBtn.addActionListener(e -> {
            optionsPanel.setRetries((Integer) retriesSpinner.getValue());
            optionsPanel.doStart();
        });
        row2.add(Box.createHorizontalStrut(8));
        row2.add(startBtn);
        midPanel.add(row2);

        // 下载进度文字
        downloadStatusLabel = new JLabel(" ");
        downloadStatusLabel.setFont(downloadStatusLabel.getFont().deriveFont(Font.BOLD, 12f));
        downloadStatusLabel.setAlignmentX(Component.LEFT_ALIGNMENT);
        midPanel.add(downloadStatusLabel);

        box.add(midPanel, BorderLayout.CENTER);
        return box;
    }

    private volatile boolean toolDownloading = false;

    /** 缓存 yt-dlp 版本，避免每次 refreshStatusLines 都启动进程 */
    private String cachedYtVer = null;
    private boolean cachedYtOk = false;
    private boolean ytVerChecked = false;

    public void refreshStatusLines() {
        if (statusLine1 == null) return;
        Font sf = new JLabel().getFont().deriveFont(13f);

        // yt-dlp 行（版本号缓存，仅首次启动进程）
        statusLine1.removeAll();
        if (!ytVerChecked) {
            try {
                util.ProcessHelper.CommandResult r = util.ProcessHelper.execute(
                        java.util.List.of(util.ProcessHelper.findYtDlp(), "--version"));
                cachedYtOk = r.isSuccess();
                cachedYtVer = cachedYtOk && !r.stdout.isEmpty() ? r.stdout.get(0).trim() : "?";
            } catch (Exception ignored) {
                cachedYtVer = "?";
            }
            ytVerChecked = true;
        }
        JLabel ytLabel = new JLabel("Yt-dlp: " + cachedYtVer);
        ytLabel.setFont(sf);
        statusLine1.add(ytLabel);
        if (!cachedYtOk) {
            JButton dl = smallBtn("Download");
            dl.addActionListener(e -> downloadTool(true, dl));
            statusLine1.add(dl);
        }

        // Proxy 行
        statusLine2.removeAll();
        JLabel px = new JLabel("Proxy: " + (util.ProxyConfig.isEnabled() ? util.ProxyConfig.getProxyString() : "N/A"));
        px.setFont(sf);
        statusLine2.add(px);

        // Cookies 行
        statusLine3.removeAll();
        String ck = mainFrame.downloader.getCookiesFromBrowser();
        if (ck == null) ck = mainFrame.downloader.getCookiesFile();
        JLabel co = new JLabel("Cookies: " + (ck != null ? ck : "N/A"));
        co.setFont(sf);
        statusLine3.add(co);

        // ffmpeg 行
        statusLine4.removeAll();
        boolean hasFfmpeg = util.ProcessHelper.isFfmpegAvailable();
        JLabel ff = new JLabel("FFmpeg: " + (hasFfmpeg ? "OK" : "N/A"));
        ff.setFont(sf);
        statusLine4.add(ff);
        if (!hasFfmpeg) {
            JButton ffBtn = smallBtn("Download");
            ffBtn.addActionListener(e -> downloadTool(false, ffBtn));
            statusLine4.add(ffBtn);
        }

        statusLine1.revalidate(); statusLine2.revalidate();
        statusLine3.revalidate(); statusLine4.revalidate();
    }

    private JButton smallBtn(String text) {
        JButton b = new JButton(text);
        b.setFont(b.getFont().deriveFont(11f));
        b.setMargin(new java.awt.Insets(1, 4, 1, 4));
        return b;
    }

    private void downloadTool(boolean ytDlp, JButton sourceBtn) {
        if (toolDownloading) {
            JOptionPane.showMessageDialog(this, "A download is already in progress.", "Busy", JOptionPane.WARNING_MESSAGE);
            return;
        }
        toolDownloading = true;
        sourceBtn.setEnabled(false);
        sourceBtn.setText("...");
        setDownloadStatus("[...] Downloading " + (ytDlp ? "yt-dlp" : "ffmpeg") + "...");
        new ui.gui.workers.BootstrapWorker(ytDlp, result -> {
            SwingUtilities.invokeLater(() -> {
                toolDownloading = false;
                setDownloadStatus(result != null && !result.startsWith("failed") ? "[+] Ready" : "[-] Failed");
                refreshStatusLines();
                mainFrame.refreshStatusBar();
            });
        }).execute();
    }

    /** 文字在中间的横向分隔线 */
    private JPanel buildSeparator(String text) {
        return new JPanel() {
            {
                setPreferredSize(new Dimension(0, 20));
                setOpaque(false);
            }
            @Override
            protected void paintComponent(Graphics g) {
                super.paintComponent(g);
                int w = getWidth(), h = getHeight();
                int mid = h / 2;
                g.setColor(Color.LIGHT_GRAY);
                g.drawLine(8, mid, w - 8, mid);

                String s = " " + text + " ";
                FontMetrics fm = g.getFontMetrics();
                int sw = fm.stringWidth(s);
                int sx = 16; // 左对齐，留边距
                g.setColor(javax.swing.UIManager.getColor("Panel.background"));
                g.fillRect(sx, mid - fm.getAscent() / 2, sw, fm.getAscent() + 2);
                g.setColor(Color.GRAY);
                g.drawString(s, sx, mid + fm.getAscent() / 2 - 2);
            }
        };
    }

    void setDownloadStatus(String text) {
        if (downloadStatusLabel != null) downloadStatusLabel.setText(text);
    }

    /** 应用 cookies 到 downloader */
    public void applyCookiesToDownloader() {
        String browser = mainFrame.downloader.getCookiesFromBrowser();
        String file = mainFrame.downloader.getCookiesFile();
        if (browser != null && !browser.isEmpty()) {
            mainFrame.downloader.setCookiesFromBrowser(browser);
        } else if (file != null && !file.isEmpty()) {
            mainFrame.downloader.setCookiesFile(file);
        }
    }
}
