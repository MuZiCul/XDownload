package ui.gui.workers;

import downloader.YtDlpDownloader;
import javax.swing.*;
import java.util.function.Consumer;

public class UpdateWorker extends SwingWorker<Boolean, Void> {
    private final YtDlpDownloader downloader;
    private final Consumer<Boolean> callback;
    public UpdateWorker(YtDlpDownloader d, Consumer<Boolean> cb) { downloader = d; callback = cb; }
    protected Boolean doInBackground() throws Exception { return downloader.updateYtDlp(); }
    protected void done() { try { callback.accept(get()); } catch (Exception e) { callback.accept(false); } }
}
