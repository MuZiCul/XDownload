package ui.gui.workers;

import util.Bootstrap;
import javax.swing.*;
import java.util.List;
import java.util.function.Consumer;

public class BootstrapWorker extends SwingWorker<String, String> {
    private final boolean ytDlp;
    private final Consumer<String> callback;

    public BootstrapWorker(boolean ytDlp, Consumer<String> cb) {
        this.ytDlp = ytDlp; this.callback = cb;
    }

    @Override
    protected String doInBackground() throws Exception {
        String label = ytDlp ? "yt-dlp" : "ffmpeg";
        publish("Connecting...");
        String result;
        if (ytDlp) {
            publish("Downloading " + label + " (~15MB)...");
            result = Bootstrap.ensureYtDlp();
        } else {
            publish("Downloading " + label + " (~80MB)...");
            result = Bootstrap.ensureFfmpeg(true);
        }
        publish(result != null && !result.startsWith("failed") ? "Done" : "Failed");
        return result;
    }

    @Override
    protected void process(List<String> chunks) {
        // progress published as strings; caller reads via callback
    }

    @Override
    protected void done() {
        try { callback.accept(get()); } catch (Exception e) { callback.accept("failed: " + e.getMessage()); }
    }
}
