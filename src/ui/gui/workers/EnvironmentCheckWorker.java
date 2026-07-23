package ui.gui.workers;

import util.NetworkDetect;
import javax.swing.*;
import java.util.function.Consumer;

public class EnvironmentCheckWorker extends SwingWorker<Boolean, Void> {
    private final Consumer<Boolean> callback;
    public EnvironmentCheckWorker(Consumer<Boolean> cb) { this.callback = cb; }
    protected Boolean doInBackground() { return NetworkDetect.isOverseas(); }
    protected void done() { try { callback.accept(get()); } catch (Exception e) { callback.accept(false); } }
}
