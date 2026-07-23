package ui.gui.workers;

import util.ProcessHelper;
import javax.swing.*;
import java.util.function.Consumer;

public class CookiesValidationWorker extends SwingWorker<ProcessHelper.CookiesValidationResult, Void> {
    private final String browser;
    private final Consumer<ProcessHelper.CookiesValidationResult> callback;

    public CookiesValidationWorker(String browser, Consumer<ProcessHelper.CookiesValidationResult> cb) {
        this.browser = browser; this.callback = cb;
    }

    protected ProcessHelper.CookiesValidationResult doInBackground() {
        return ProcessHelper.validateCookiesFromBrowser(browser);
    }

    protected void done() {
        try { callback.accept(get()); } catch (Exception e) { callback.accept(null); }
    }
}
