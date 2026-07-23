package ui.gui.workers;

import util.Bootstrap;
import javax.swing.*;
import java.util.function.Consumer;

public class BootstrapWorker extends SwingWorker<String, Integer> {
    private final boolean ytDlp; // true=yt-dlp, false=ffmpeg
    private final Consumer<String> callback;

    public BootstrapWorker(boolean ytDlp, Consumer<String> cb) {
        this.ytDlp = ytDlp; this.callback = cb;
    }

    protected String doInBackground() throws Exception {
        if (ytDlp) return Bootstrap.ensureYtDlp();
        return Bootstrap.ensureFfmpeg(true);
    }

    protected void done() {
        try { callback.accept(get()); } catch (Exception e) { callback.accept("failed: " + e.getMessage()); }
    }
}
