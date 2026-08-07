import { useState, useEffect } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Toaster, toast } from "sonner";
import TabBar from "./components/layout/TabBar";
import StatusBar from "./components/layout/StatusBar";
import GlobalDownloadBar from "./components/layout/GlobalDownloadBar";
import DownloadPage from "./components/download/DownloadPage";
import SettingsPage from "./components/settings/SettingsPage";
import HistoryPage from "./components/history/HistoryPage";
import AboutPage from "./components/about/AboutPage";
import DisclaimerPage from "./components/about/DisclaimerPage";
import { CONTENT } from "./components/about/DisclaimerPage";
import { initDownloadStore } from "./lib/downloadStore";
import { initI18n, useI18n } from "./lib/i18n";
import {
  acceptDisclaimer,
  checkUpdate,
  checkYtdlpUpdate,
  checkFfmpegUpdate,
  downloadUpdate,
  getDisclaimerAccepted,
  getUninstallInfo,
  installUpdate,
  loadSettings,
  openUninstallPanel,
  uninstallApp,
} from "./lib/bindings";
import type {
  UpdateCheckResult,
  YtdlpUpdateResult,
  FfmpegUpdateResult,
} from "./lib/bindings";
import { listen } from "@tauri-apps/api/event";
import { ArrowUpRight, Download, Loader2, Trash2, X } from "lucide-react";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: { staleTime: 30_000, retry: 1 },
  },
});

type Tab = "download" | "settings" | "history" | "about" | "disclaimer";

function ToolCard({
  label,
  color,
  update,
  button,
}: {
  label: string;
  color: { bg: string; text: string; btn: string; btnHover: string };
  update: YtdlpUpdateResult | FfmpegUpdateResult | null;
  button: React.ReactNode;
}) {
  const { t } = useI18n();
  if (!update) return null;

  return (
    <div className={`${color.bg} rounded-xl px-4 py-3 mb-3 text-left`}>
      <p className="text-xs font-semibold text-gray-500 mb-1">{label}</p>

      {update.not_installed ? (
        <div>
          <p className="text-sm font-medium text-red-600 mb-1">
            {t("tools.statusNotInstalled")}
          </p>
          <p className="text-[11px] text-gray-400 mb-3">
            {t("app.toolNotInstalled", { label })}
          </p>
          {button}
        </div>
      ) : (
        <div className="flex items-center justify-between">
          <div>
            <p className={`text-lg font-bold ${color.text}`}>
              v{update.latest_version}
            </p>
            <p className="text-[11px] text-gray-400">
              {t("app.currentVersion", { ver: update.local_version })}
            </p>
          </div>
          {button}
        </div>
      )}
    </div>
  );
}

const APP_COLOR = {
  bg: "bg-blue-50/60",
  text: "text-blue-600",
  btn: "bg-blue-500",
  btnHover: "hover:bg-blue-600",
} as const;

const YTDLP_COLOR = {
  bg: "bg-emerald-50/60",
  text: "text-emerald-600",
  btn: "bg-emerald-500",
  btnHover: "hover:bg-emerald-600",
} as const;

const FFMPEG_COLOR = {
  bg: "bg-orange-50/60",
  text: "text-orange-600",
  btn: "bg-orange-500",
  btnHover: "hover:bg-orange-600",
} as const;

function App() {
  const { lang, t } = useI18n();
  const [activeTab, setActiveTab] = useState<Tab>("download");
  const [appUpdate, setAppUpdate] = useState<UpdateCheckResult | null>(null);
  const [ytdlpUpdate, setYtdlpUpdate] = useState<YtdlpUpdateResult | null>(null);
  const [ffmpegUpdate, setFfmpegUpdate] = useState<FfmpegUpdateResult | null>(null);

  // --- Forced disclaimer (first launch) ---
  const [disclaimerAccepted, setDisclaimerAccepted] = useState<boolean | null>(null);
  const [showDeclineConfirm, setShowDeclineConfirm] = useState(false);
  const [uninstalling, setUninstalling] = useState(false);

  // Check disclaimer acceptance on startup (takes priority over update modal)
  useEffect(() => {
    let active = true;
    getDisclaimerAccepted()
      .then((accepted) => {
        if (active) setDisclaimerAccepted(accepted);
      })
      .catch(() => {
        // If the check fails, show the disclaimer to be safe.
        if (active) setDisclaimerAccepted(false);
      });
    return () => {
      active = false;
    };
  }, []);

  // Initialize the global download store (event listeners + state recovery).
  useEffect(() => {
    initDownloadStore();
  }, []);

  // Load the persisted UI language.
  useEffect(() => {
    initI18n();
  }, []);

  // Auto-check for updates on startup (app + yt-dlp + ffmpeg)
  useEffect(() => {
    Promise.all([
      checkUpdate().then((r) => {
        if (r.has_update && !r.error) setAppUpdate(r);
      }),
      checkYtdlpUpdate().then((r) => {
        if (r.has_update || r.not_installed) setYtdlpUpdate(r);
      }),
      checkFfmpegUpdate().then((r) => {
        if (r.has_update || r.not_installed) setFfmpegUpdate(r);
      }),
    ]).catch(() => {});
  }, []);

  const showModal =
    disclaimerAccepted !== false &&
    (appUpdate !== null || ytdlpUpdate !== null || ffmpegUpdate !== null);

  const closeModal = () => {
    setAppUpdate(null);
    setYtdlpUpdate(null);
    setFfmpegUpdate(null);
    setUpdatePhase("idle");
    setUpdatePercent(0);
    setUpdatePath(null);
  };

  // --- In-app updater flow: download installer → install ---
  type UpdatePhase = "idle" | "downloading" | "downloaded" | "installing";
  const [updatePhase, setUpdatePhase] = useState<UpdatePhase>("idle");
  const [updatePercent, setUpdatePercent] = useState(0);
  const [updatePath, setUpdatePath] = useState<string | null>(null);

  const handleDownloadUpdate = async () => {
    if (!appUpdate?.download_url || updatePhase === "downloading") return;
    setUpdatePhase("downloading");
    setUpdatePercent(0);
    const unlisten = await listen<any>("update-download-progress", (e) => {
      const pct = Number(e.payload?.percent ?? 0);
      setUpdatePercent(Math.min(Math.max(pct, 0), 100));
    });
    try {
      const path = await downloadUpdate(appUpdate.download_url);
      setUpdatePath(path);
      setUpdatePhase("downloaded");
    } catch (err: any) {
      toast.error(t("app.downloadFail", { err }));
      setUpdatePhase("idle");
      setUpdatePercent(0);
    } finally {
      unlisten();
    }
  };

  const handleInstallUpdate = async () => {
    if (!updatePath || updatePhase !== "downloaded") return;
    setUpdatePhase("installing");
    try {
      await installUpdate(updatePath);
      // The app exits after launching the installer.
    } catch (err: any) {
      toast.error(t("app.installFail", { err }));
      setUpdatePhase("downloaded");
    }
  };

  const d = CONTENT[lang];

  // Accept → persist and enter the app.
  const handleAcceptDisclaimer = async () => {
    try {
      await acceptDisclaimer();
      setDisclaimerAccepted(true);
    } catch (err: any) {
      toast.error(`${err}`);
    }
  };

  // Decline (after second confirmation) → uninstall and exit.
  const handleConfirmDecline = async () => {
    if (uninstalling) return;
    setUninstalling(true);
    try {
      const info = await getUninstallInfo();
      if (info.installed) {
        const handled = await uninstallApp();
        // handled === true → uninstaller launched, the app is exiting.
        if (!handled) {
          // Installed entry exists but no usable UninstallString / launch failed →
          // fall back to the system uninstall panel.
          await openUninstallPanel();
          toast.success(d.uninstall.panelHint);
          setShowDeclineConfirm(false);
        }
        return;
      }
      // Not registered (dev / portable build) → open the system uninstall panel.
      await openUninstallPanel();
      toast.success(d.uninstall.panelHint);
      setShowDeclineConfirm(false);
    } catch (err: any) {
      toast.error(`${err}`);
      setShowDeclineConfirm(false);
    } finally {
      setUninstalling(false);
    }
  };

  return (
    <QueryClientProvider client={queryClient}>
      <div className="flex flex-col h-screen overflow-hidden bg-[#fafafa]">
        <TabBar activeTab={activeTab} onTabChange={setActiveTab} />

        <main className="flex-1 overflow-auto min-h-0">
          {/* DownloadPage stays mounted so its state (parsed video info,
              format selection, download progress) survives tab switches. */}
          <div className={activeTab === "download" ? "" : "hidden"}>
            <DownloadPage />
          </div>
          {activeTab === "settings" && <SettingsPage />}
          {activeTab === "history" && <HistoryPage />}
          {activeTab === "about" && <AboutPage />}
          {activeTab === "disclaimer" && <DisclaimerPage />}
        </main>

        <StatusBar />
      </div>

      <Toaster
        position="top-right"
        richColors
        expand
        visibleToasts={5}
        gap={8}
        toastOptions={{
          style: {
            minHeight: "52px",
            padding: "10px 16px",
            whiteSpace: "pre-line",
          },
        }}
      />

      {/* Update available modal — only manual close */}
      {showModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" />

          <div className="relative z-10 bg-white/85 backdrop-blur-xl rounded-2xl shadow-2xl p-8 w-[400px] border border-white/40 text-center">
            <button
              className="absolute top-3 right-3 text-gray-400 hover:text-gray-600 transition-colors"
              onClick={closeModal}
              aria-label={t("common.close")}
            >
              <X size={18} />
            </button>

            <p className="text-base font-semibold text-gray-800 mb-5">
              {appUpdate ? t("app.newVersion") : t("app.toolStatus")}
            </p>

            {/* App update */}
            {appUpdate && (
              <div className="bg-blue-50/60 rounded-xl px-4 py-3 mb-3 text-left">
                <p className="text-xs font-semibold text-gray-500 mb-1">
                  XDownload
                </p>
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <p className="text-lg font-bold text-blue-600">
                      v{appUpdate.latest_version}
                    </p>
                    <p className="text-[11px] text-gray-400">
                      {t("app.currentVersion", { ver: appUpdate.current_version })}
                    </p>
                  </div>
                  {appUpdate.download_url ? (
                    updatePhase === "idle" && (
                      <button
                        className={`px-4 py-2 text-xs rounded-lg ${APP_COLOR.btn} text-white font-medium ${APP_COLOR.btnHover} transition-colors inline-flex items-center gap-1`}
                        onClick={handleDownloadUpdate}
                      >
                        <Download size={12} />
                        {t("app.downloadUpdate")}
                      </button>
                    )
                  ) : (
                    <a
                      href={
                        appUpdate.url ??
                        "https://github.com/MuZiCul/XDownload/releases"
                      }
                      target="_blank"
                      rel="noopener noreferrer"
                      className={`px-4 py-2 text-xs rounded-lg ${APP_COLOR.btn} text-white font-medium ${APP_COLOR.btnHover} transition-colors inline-flex items-center gap-1`}
                      onClick={() => setAppUpdate(null)}
                    >
                      {t("app.download")}
                      <ArrowUpRight size={12} />
                    </a>
                  )}
                </div>

                {/* In-app download progress */}
                {appUpdate.download_url && updatePhase === "downloading" && (
                  <div className="mt-2 flex items-center gap-2">
                    <div className="flex-1 h-1.5 bg-white/70 rounded-full overflow-hidden">
                      <div
                        className="h-full rounded-full transition-all duration-200"
                        style={{
                          width: `${Math.max(updatePercent, 2)}%`,
                          background: "linear-gradient(90deg, #3b82f6, #6366f1)",
                        }}
                      />
                    </div>
                    <span className="text-[11px] tabular-nums text-gray-500 shrink-0">
                      {updatePercent}%
                    </span>
                  </div>
                )}

                {/* Install button */}
                {appUpdate.download_url && updatePhase === "downloaded" && (
                  <div className="mt-2 flex items-center justify-end">
                    <button
                      className="px-4 py-2 text-xs rounded-lg bg-emerald-500 text-white font-medium hover:bg-emerald-600 transition-colors inline-flex items-center gap-1"
                      onClick={handleInstallUpdate}
                    >
                      <Download size={12} />
                      {t("app.installUpdate")}
                    </button>
                  </div>
                )}

                {/* Installing */}
                {appUpdate.download_url && updatePhase === "installing" && (
                  <div className="mt-2 flex items-center justify-end text-[11px] text-gray-500 gap-1.5">
                    <Loader2 size={12} className="animate-spin" />
                    {t("app.installing")}
                  </div>
                )}
              </div>
            )}

            {/* yt-dlp update */}
            <ToolCard
              label="yt-dlp"
              color={YTDLP_COLOR}
              update={ytdlpUpdate}
              button={
                <button
                  className={`px-4 py-2 text-xs rounded-lg ${YTDLP_COLOR.btn} text-white font-medium ${YTDLP_COLOR.btnHover} transition-colors`}
                  onClick={() => {
                    closeModal();
                    setActiveTab("settings");
                  }}
                >
                  {t("app.goToSettings")}
                </button>
              }
            />

            {/* ffmpeg update */}
            <ToolCard
              label="ffmpeg"
              color={FFMPEG_COLOR}
              update={ffmpegUpdate}
              button={
                <button
                  className={`px-4 py-2 text-xs rounded-lg ${FFMPEG_COLOR.btn} text-white font-medium ${FFMPEG_COLOR.btnHover} transition-colors`}
                  onClick={() => {
                    closeModal();
                    setActiveTab("settings");
                  }}
                >
                  {t("app.goToSettings")}
                </button>
              }
            />
          </div>
        </div>
      )}

      {/* Forced disclaimer modal (first launch) — no close button, no ESC, no
          backdrop click; only Accept or I Don't Accept */}
      {disclaimerAccepted === false && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" />

          <div className="relative z-10 bg-white/85 backdrop-blur-xl rounded-2xl shadow-2xl w-[580px] max-w-[90vw] border border-white/40">
            <div className="px-7 pt-6 pb-2">
              <h2 className="text-lg font-semibold text-gray-800 text-center">
                {d.title}
              </h2>
            </div>

            {/* Full terms — scrollable */}
            <div className="px-7 py-4 max-h-[45vh] overflow-y-auto">
              <ol className="list-decimal pl-5 space-y-2 text-[13px] leading-relaxed text-gray-700 text-left">
                {d.items.map((item, i) => (
                  <li key={i}>{item}</li>
                ))}
              </ol>
              <p className="mt-3 text-[13px] font-medium text-gray-800">
                {d.footer}
              </p>
            </div>

            <div className="px-7 py-5 border-t border-zinc-200/70 flex gap-2 justify-end">
              <button
                className="px-4 py-2 text-xs rounded-lg bg-red-500 text-white font-medium hover:bg-red-600 transition-colors"
                onClick={() => setShowDeclineConfirm(true)}
                disabled={uninstalling}
              >
                {d.disclaimer.decline}
              </button>
              <button
                className="px-4 py-2 text-xs rounded-lg bg-blue-500 text-white font-medium hover:bg-blue-600 transition-colors"
                onClick={handleAcceptDisclaimer}
                disabled={uninstalling}
              >
                {d.disclaimer.accept}
              </button>
            </div>
          </div>

          {/* Decline → second confirmation */}
          {showDeclineConfirm && (
            <div className="fixed inset-0 z-[60] flex items-center justify-center">
              <div
                className="absolute inset-0 bg-black/40 backdrop-blur-sm"
                onClick={() => !uninstalling && setShowDeclineConfirm(false)}
              />
              <div
                className="relative z-10 bg-white/85 backdrop-blur-xl rounded-2xl shadow-2xl w-[400px] border border-white/40 p-6 text-center"
                onClick={(e) => e.stopPropagation()}
              >
                <h3 className="text-sm font-semibold text-zinc-900 mb-3">
                  {d.disclaimer.declineModalTitle}
                </h3>
                <p className="text-xs text-zinc-500 mb-5 leading-relaxed">
                  {d.disclaimer.declineModalBody}
                </p>
                <div className="flex gap-2 justify-end">
                  <button
                    className="btn"
                    onClick={() => setShowDeclineConfirm(false)}
                    disabled={uninstalling}
                  >
                    {d.disclaimer.cancel}
                  </button>
                  <button
                    className="btn btn-danger flex items-center gap-1.5"
                    onClick={handleConfirmDecline}
                    disabled={uninstalling}
                  >
                    {uninstalling ? (
                      <>
                        <Loader2 size={13} className="animate-spin" />
                        {d.disclaimer.confirming}
                      </>
                    ) : (
                      <>
                        <Trash2 size={13} />
                        {d.disclaimer.confirm}
                      </>
                    )}
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      )}
    </QueryClientProvider>
  );
}

export default App;
