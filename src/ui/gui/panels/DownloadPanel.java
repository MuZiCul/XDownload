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

    private final MainFrame mainFrame;
    private final URLInputPanel urlInputPanel;
    private final VideoInfoPanel videoInfoPanel;
    private final FormatTablePanel formatTablePanel;
    private final DownloadOptionsPanel optionsPanel;
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
        optionsPanel = new DownloadOptionsPanel(this);

        JPanel leftPanel = new JPanel(new BorderLayout());
        leftPanel.add(videoInfoPanel, BorderLayout.NORTH);
        leftPanel.add(formatTablePanel, BorderLayout.CENTER);

        JSplitPane splitPane = new JSplitPane(JSplitPane.HORIZONTAL_SPLIT, leftPanel, optionsPanel);
        splitPane.setResizeWeight(0.6);

        // 底部：进度条
        progressPanel = new ProgressPanel();

        add(urlInputPanel, BorderLayout.NORTH);
        add(splitPane, BorderLayout.CENTER);
        add(progressPanel, BorderLayout.SOUTH);
    }

    /** 用户点击 Fetch Info */
    public void fetchVideoInfo(String url) {
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

    /** 格式选中 */
    public void onFormatSelected(String formatId) {
        if (currentConfig != null) {
            currentConfig.setFormatId(formatId);
        }
    }

    /** 开始下载 */
    public void startDownload(DownloadConfig config) {
        this.currentConfig = config;
        progressPanel.reset();
        progressPanel.setStatus("[...] " + I18n.get("prog.downloading"));
        optionsPanel.setDownloading(true);
        downloadWorker = new DownloadWorker(mainFrame.downloader, config, this);
        downloadWorker.execute();
    }

    /** 取消下载 */
    public void cancelDownload() {
        if (downloadWorker != null && !downloadWorker.isDone()) {
            downloadWorker.cancel(true);
            mainFrame.downloader.cancel();
            progressPanel.setStatus("[-] " + I18n.get("prog.cancelled"));
            optionsPanel.setDownloading(false);
        }
    }

    /** 下载进度回调 */
    public void onDownloadProgress(DownloadProgress progress) {
        progressPanel.updateProgress(progress);
    }

    /** 下载完成 */
    public void onDownloadComplete(boolean success, String outputPath) {
        optionsPanel.setDownloading(false);
        if (success) {
            progressPanel.setComplete();
            JOptionPane.showMessageDialog(this,
                    I18n.get("prog.complete.msg") + outputPath,
                    I18n.get("prog.done.title"), JOptionPane.INFORMATION_MESSAGE);
        } else {
            progressPanel.setError("[-] " + I18n.get("prog.failed"));
        }
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
