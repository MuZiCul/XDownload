package ui.gui.workers;

import util.ProxyConfig;
import javax.swing.*;
import java.util.function.Consumer;

public class ProxyTestWorker extends SwingWorker<ProxyConfig.ProxyTestResult, Void> {
    private final String host;
    private final int port;
    private final Consumer<ProxyConfig.ProxyTestResult> callback;

    public ProxyTestWorker(String host, int port, Consumer<ProxyConfig.ProxyTestResult> cb) {
        this.host = host; this.port = port; this.callback = cb;
    }

    protected ProxyConfig.ProxyTestResult doInBackground() {
        ProxyConfig.setProxy(host, port);
        return ProxyConfig.testProxy();
    }

    protected void done() {
        try { callback.accept(get()); } catch (Exception e) { callback.accept(null); }
    }
}
