import { useState, useEffect, useCallback, useRef } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Toaster, toast } from "sonner";
import TabBar from "./components/layout/TabBar";
import StatusBar from "./components/layout/StatusBar";
import DownloadPage from "./components/download/DownloadPage";
import SettingsPage from "./components/settings/SettingsPage";
import HistoryPage from "./components/history/HistoryPage";
import AboutPage from "./components/about/AboutPage";
import DisclaimerPage from "./components/about/DisclaimerPage";
import { CONTENT } from "./lib/disclaimerContent";
import { initDownloadStore, enqueueDownloadGlobal, buildBatchConfig } from "./lib/downloadStore";
import { initBookmarkSync } from "./lib/bookmarkSync";
import BookmarkSyncModal from "./components/settings/BookmarkSyncModal";
import DuplicateDownloadModal, {
  type DuplicateItem,
} from "./components/common/DuplicateDownloadModal";
import type { DownloadHistoryItem } from "./lib/types";
import { TaskSource } from "./lib/types";
import { initI18n, useI18n } from "./lib/i18n";
import {
  acceptDisclaimer,
  buildProxyUrl,
  checkUpdateNetwork,
  checkYtdlpUpdate,
  checkFfmpegUpdate,
  cleanupUpdaterTemp,
  getDisclaimerAccepted,
  getUninstallInfo,
  hasActiveTasks,
  loadSettings,
  openUninstallPanel,
  quitApp,
  uninstallApp,
} from "./lib/bindings";
import type {
  YtdlpUpdateResult,
  FfmpegUpdateResult,
} from "./lib/bindings";
import type { GitHubReachability } from "./lib/types";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getPrivacyMode, setPrivacyMode, initPrivacyMode } from "./lib/privacyMode";
import { ArrowUpRight, Download, Loader2, Save, Power, Minimize2, Trash2, X } from "lucide-react";

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
  const [appUpdate, setAppUpdate] = useState<{
    version: string;
    currentVersion: string;
  } | null>(null);
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
    initPrivacyMode();
    initBookmarkSync();
  }, []);

  // 系统托盘菜单的隐私开关：后端 emit toggle-privacy-mode → 前端切换。
  useEffect(() => {
    const unlisten = listen<any>("toggle-privacy-mode", () => {
      setPrivacyMode(!getPrivacyMode());
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // 支持从下载页弹窗「前往任务」等跨页跳转。
  useEffect(() => {
    const handler = (e: Event) => {
      const tab = (e as CustomEvent).detail;
      if (tab === "download" || tab === "settings" || tab === "history") {
        setActiveTab(tab);
      }
    };
    window.addEventListener("switch-tab", handler);
    return () => window.removeEventListener("switch-tab", handler);
  }, []);

  // ---- 退出确认流程 ----
  // 后端在以下场景 emit quit-requested：窗口 X（source=close）、托盘退出
  // （source=tray）；设置页退出按钮 dispatch 同事件（source=settings）。
  // 弹窗按钮按条件简化：
  //   simple=true（无任务，或队列持久化开启=任务已自动保存）
  //     → 只显示「最小化到托盘」「退出」，不询问保存进度；
  //   simple=false（有任务且队列持久化关闭）
  //     → 显示完整按钮（保存进度并退出 / 直接退出 / 最小化到托盘 / 取消）。
  const [quitDialog, setQuitDialog] = useState<{
    source: "close" | "tray" | "settings";
    simple: boolean;
  } | null>(null);

  // 深链（浏览器扩展）已下载查重：待用户逐条选择「重新下载 / 取消」的链接。
  // 弹窗被强制处理完所有条目后自动卸载（与批量下载的 dup 弹窗行为一致）。
  const [deepDups, setDeepDups] = useState<DuplicateItem[]>([]);

  useEffect(() => {
    const onBackend = (e: { payload?: { source?: string } }) => {
      const source = e.payload?.source === "tray" ? "tray" : e.payload?.source === "settings" ? "settings" : "close";
      handleQuitRequest(source);
    };
    const onCustom = (e: Event) => {
      const source = (e as CustomEvent)?.detail?.source;
      handleQuitRequest(source === "tray" ? "tray" : source === "settings" ? "settings" : "close");
    };
    const unlisten = listen<any>("quit-requested", onBackend);
    window.addEventListener("quit-requested", onCustom);
    // 深链已下载查重：后端 process_deep_link_batch 收集重复链接 → downloadStore
    // 转发 deep-link-duplicates → 这里弹窗让用户逐条选择重新下载/取消。
    const onDeepDups = (e: Event) => {
      const dups = (e as CustomEvent)?.detail;
      if (Array.isArray(dups) && dups.length > 0) {
        setDeepDups(dups);
      }
    };
    window.addEventListener("deep-link-duplicates", onDeepDups);
    return () => {
      unlisten.then((fn) => fn());
      window.removeEventListener("quit-requested", onCustom);
      window.removeEventListener("deep-link-duplicates", onDeepDups);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleQuitRequest = (source: "close" | "tray" | "settings") => {
    // 无任务或队列持久化开启（任务已自动保存）→ 精简弹窗（close/settings：
    // 最小化/退出；tray：仅退出），不询问保存进度；仅「有任务且队列持久化
    // 关闭」才显示完整按钮。
    Promise.all([hasActiveTasks(), loadSettings().catch(() => null)])
      .then(([active, s]) => {
        const simple = !active || !!s?.queue_persist;
        setQuitDialog({ source, simple });
      })
      .catch(() => {
        // 查询失败按精简处理：仍有退出/最小化选项，不给用户制造障碍。
        setQuitDialog({ source, simple: true });
      });
  };

  const doQuit = (saveProgress: boolean) => {
    setQuitDialog(null);
    quitApp(saveProgress);
  };

  // 最小化到托盘：不退出应用，隐藏主窗口，下载任务继续在后台运行。
  const doHideToTray = () => {
    setQuitDialog(null);
    getCurrentWindow().hide().catch(() => {});
  };

  // 深链 dup 弹窗「重新下载」：立即入队（autoStart，来源=深链）后移除该项。
  const handleDeepDupRedownload = async (item: DuplicateItem) => {
    const s = await loadSettings().catch(() => null);
    try {
      await enqueueDownloadGlobal(buildBatchConfig(item.url, item.video_id, s), {
        autoStart: true,
        source: TaskSource.Deep,
      });
      setDeepDups((prev) => prev.filter((x) => x.url !== item.url));
    } catch (err: any) {
      toast.warning(String(err?.message ?? err));
    }
  };

  // 深链 dup 弹窗「取消」：直接移除，不入队。
  const handleDeepDupCancel = (item: DuplicateItem) => {
    setDeepDups((prev) => prev.filter((x) => x.url !== item.url));
  };

  // Load the persisted UI language.
  useEffect(() => {
    initI18n();
  }, []);

  /** Check for updates with the same network fallback used elsewhere:
   *  direct request first; if it fails and a proxy is configured, retry
   *  through the proxy. Returns the Update (or null when up to date).
   *  Must be declared BEFORE the startup-check effect below (TDZ). */
  const checkForUpdate = useCallback(async () => {
    try {
      return await check();
    } catch {
      // Direct failed — fall back to the configured proxy if any.
      try {
        const settings = await loadSettings();
        const proxy = buildProxyUrl(settings);
        if (!proxy) throw new Error("no proxy");
        return await check({ proxy });
      } catch {
        throw new Error("update network failed");
      }
    }
  }, []);

  // Auto-check for updates on startup (app + yt-dlp + ffmpeg)
  useEffect(() => {
    Promise.all([
      checkForUpdate().then((update) => {
        if (update) {
          setAppUpdate({
            version: update.version,
            currentVersion: update.currentVersion,
          });
        }
      }),
      checkYtdlpUpdate().then((r) => {
        if (r.has_update || r.not_installed) setYtdlpUpdate(r);
      }),
      checkFfmpegUpdate().then((r) => {
        if (r.has_update || r.not_installed) setFfmpegUpdate(r);
      }),
    ]).catch(() => {});
  }, [checkForUpdate]);

  // Open the app update modal on demand (e.g. About page "check for updates"
  // → 有更新直接弹拟态窗), reusing the same update modal shown at startup.
  useEffect(() => {
    const handler = (e: Event) => {
      const detail = (e as CustomEvent).detail as
        | { version: string; currentVersion: string }
        | undefined;
      if (!detail) return;
      stopRef.current = false;
      setAppUpdate(detail);
      setUpdatePhase("idle");
      setUpdatePercent(0);
      setUpdateError(null);
      setNetworkResult(null);
      updateRef.current = null;
    };
    window.addEventListener("open-update-modal", handler);
    return () => window.removeEventListener("open-update-modal", handler);
  }, []);

  // --- In-app updater flow ---
  // checking(网络预检) → downloading(下载) → downloaded(待安装) → installing → relaunch。
  // 失败：failed（网络不通或下载失败）；下载超时（3 分钟）：timeout。
  type UpdatePhase =
    | "idle"
    | "checking"
    | "downloading"
    | "downloaded"
    | "installing"
    | "failed"
    | "timeout";
  const [updatePhase, setUpdatePhase] = useState<UpdatePhase>("idle");
  const [updatePercent, setUpdatePercent] = useState(0);
  /** 下载/安装失败信息（仅用于 failed 状态展示）。 */
  const [updateError, setUpdateError] = useState<string | null>(null);
  /** 网络预检结果（仅用于 failed 状态展示）。 */
  const [networkResult, setNetworkResult] = useState<GitHubReachability | null>(null);
  /** 当前 Update 对象（download 后 install 复用）。 */
  const updateRef = useRef<Update | null>(null);
  /** 停止令牌：用户点了「停止更新/取消」后置位，后台 download() 完成后不再更新 UI。 */
  const stopRef = useRef(false);

  /** 下载超时阈值：3 分钟。超时提示用户去 GitHub 手动下载。 */
  const UPDATE_TIMEOUT_MS = 3 * 60 * 1000;

  useEffect(() => {
    if (updatePhase !== "downloading") return;
    const timer = setTimeout(() => setUpdatePhase("timeout"), UPDATE_TIMEOUT_MS);
    return () => clearTimeout(timer);
  }, [updatePhase]);

  const showModal =
    disclaimerAccepted !== false &&
    updatePhase === "idle" &&
    (appUpdate !== null || ytdlpUpdate !== null || ffmpegUpdate !== null);

  const closeModal = () => {
    stopRef.current = true;
    setAppUpdate(null);
    setYtdlpUpdate(null);
    setFfmpegUpdate(null);
    setUpdatePhase("idle");
    setUpdatePercent(0);
    setUpdateError(null);
    setNetworkResult(null);
    updateRef.current = null;
  };

  /** 主弹窗「下载更新」：先做 GitHub 网络预检，再进入下载流程。 */
  const handleDownloadUpdate = async () => {
    if (
      updatePhase === "downloading" ||
      updatePhase === "downloaded" ||
      updatePhase === "installing"
    )
      return;
    stopRef.current = false;
    setUpdateError(null);
    setNetworkResult(null);
    updateRef.current = null;
    setUpdatePercent(0);
    setUpdatePhase("checking");

    let net: GitHubReachability | null = null;
    try {
      net = await checkUpdateNetwork();
    } catch {
      net = null; // 检测命令异常按不可达处理，让用户选择配置代理或继续下载。
    }
    // 检测期间用户已关闭弹窗 → 不再推进。
    if (stopRef.current) return;
    // 检测命令异常时 net 为 null，兜底为「全部不可达」，保证 failed 弹窗有内容。
    setNetworkResult(
      net ?? {
        direct_ok: false,
        proxy_configured: false,
        proxy_ok: false,
        reachable: false,
      }
    );
    if (!net?.reachable) {
      setUpdatePhase("failed");
      return;
    }
    await runUpdateDownload();
  };

  /** 实际下载（只下载不安装，下载完成后停在「待安装」等待用户确认）。 */
  const runUpdateDownload = async () => {
    setUpdateError(null);
    setUpdatePercent(0);
    setUpdatePhase("downloading");
    try {
      const update = await checkForUpdate();
      if (!update) {
        // 手动打开弹窗时若已是最新（自动检查有延迟），直接关闭。
        setAppUpdate(null);
        setUpdatePhase("idle");
        setUpdatePercent(0);
        return;
      }
      updateRef.current = update;
      setAppUpdate({
        version: update.version,
        currentVersion: update.currentVersion,
      });
      let downloaded = 0;
      let total: number | undefined;
      await update.download((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total && total > 0) {
            const pct = Math.round((downloaded / total) * 100);
            setUpdatePercent(Math.min(Math.max(pct, 0), 100));
          }
        }
      });
      // 用户已点「停止更新」→ 不再推进 UI（后台下载虽无法中断，但界面保持停止状态）。
      if (stopRef.current) return;
      // 下载完成 → 停在「待安装」，由用户点击安装（install 不再自动 relaunch）。
      setUpdatePercent(100);
      setUpdatePhase("downloaded");
    } catch (err: any) {
      if (stopRef.current) return;
      setUpdateError(t("app.downloadFail", { err }));
      setUpdatePhase("failed");
    }
  };

  /** 「继续下载」：忽略网络预检结果，直接尝试下载。 */
  const handleContinueDownload = () => {
    stopRef.current = false;
    setNetworkResult(null);
    runUpdateDownload();
  };

  /** 「配置代理」：关闭更新弹窗并跳转到设置页。 */
  const handleGoProxy = () => {
    closeModal();
    setActiveTab("settings");
  };

  /** 「下载完成 → 安装更新」：install + relaunch。 */
  const handleInstallUpdate = async () => {
    if (updatePhase !== "downloaded") return;
    setUpdateError(null);
    setUpdatePhase("installing");
    try {
      let update = updateRef.current;
      if (!update) {
        update = await checkForUpdate();
        if (!update) {
          setUpdateError(t("app.installFail", { err: "no update" }));
          setUpdatePhase("failed");
          return;
        }
      }
      await update.install();
      // 安装完毕；relaunch 重启以加载新版本。
      await relaunch();
    } catch (err: any) {
      if (stopRef.current) return;
      setUpdateError(t("app.installFail", { err }));
      setUpdatePhase("failed");
    }
  };

  /** 「停止更新 / 取消」：清理 updater 临时缓存，回到主弹窗。 */
  const handleStopUpdate = async () => {
    stopRef.current = true;
    try {
      await cleanupUpdaterTemp();
    } catch {
      // 清理失败不阻塞流程。
    }
    updateRef.current = null;
    setUpdateError(null);
    setNetworkResult(null);
    setUpdatePhase("idle");
    setUpdatePercent(0);
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
          {activeTab === "history" && (
            <HistoryPage
              onRedownload={(item: DownloadHistoryItem) => {
                // Switch to the download tab and let DownloadPage fill the
                // info card and start the download automatically.
                setActiveTab("download");
                window.dispatchEvent(
                  new CustomEvent("history-redownload", { detail: item })
                );
              }}
            />
          )}
          {activeTab === "about" && <AboutPage />}
          {activeTab === "disclaimer" && <DisclaimerPage />}
        </main>

        <StatusBar />
      </div>

      {/* 书签同步全局模态：同步中无法关闭，跨 tab 保持显示。 */}
      <BookmarkSyncModal />

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
                <div>
                  <p className="text-lg font-bold text-blue-600">
                    v{appUpdate.version}
                  </p>
                  <p className="text-[11px] text-gray-400">
                    {t("app.currentVersion", { ver: appUpdate.currentVersion })}
                  </p>

                  <div className="flex items-center gap-2 mt-3">
                    <button
                      className={`px-4 py-2 text-xs rounded-lg ${APP_COLOR.btn} text-white font-medium ${APP_COLOR.btnHover} transition-colors inline-flex items-center gap-1`}
                      onClick={handleDownloadUpdate}
                    >
                      <Download size={12} />
                      {t("app.downloadUpdate")}
                    </button>
                    <a
                      href="https://github.com/MuZiCul/XDownload/releases"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="px-3 py-2 text-xs rounded-lg border border-blue-200 text-blue-500 hover:bg-blue-50 hover:underline inline-flex items-center gap-0.5 transition-colors"
                    >
                      {t("app.download")}
                      <ArrowUpRight size={11} />
                    </a>
                  </div>
                </div>
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

      {/* Updating modal — network pre-check / download / install / timeout / failed */}
      {updatePhase !== "idle" && appUpdate && (
        <div className="fixed inset-0 z-[60] flex items-center justify-center">
          <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" />

          <div className="relative z-10 bg-white/85 backdrop-blur-xl rounded-2xl shadow-2xl p-8 w-[400px] border border-white/40 text-center">
            <p className="text-base font-semibold text-gray-800 mb-5">
              {updatePhase === "checking"
                ? t("app.checkingTitle")
                : t("app.updating")}
            </p>

            {/* Network pre-check in progress */}
            {updatePhase === "checking" && (
              <div className="flex flex-col items-center gap-2">
                <Loader2 size={18} className="animate-spin text-blue-500" />
                <p className="text-xs text-gray-600">{t("app.checkingNetwork")}</p>
              </div>
            )}

            {/* Download progress */}
            {updatePhase === "downloading" && (
              <div className="flex items-center gap-2">
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

            {/* Downloaded — waiting for the user to install */}
            {updatePhase === "downloaded" && (
              <div>
                <p className="text-xs text-gray-600 mb-4">{t("app.downloadReady")}</p>
                <div className="flex items-center justify-center gap-2">
                  <button
                    className={`px-4 py-2 text-xs rounded-lg ${APP_COLOR.btn} text-white font-medium ${APP_COLOR.btnHover} transition-colors inline-flex items-center gap-1`}
                    onClick={handleInstallUpdate}
                  >
                    <Download size={12} />
                    {t("app.installUpdate")}
                  </button>
                  <button
                    className="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-500 hover:bg-gray-50 transition-colors"
                    onClick={handleStopUpdate}
                  >
                    {t("common.cancel")}
                  </button>
                </div>
              </div>
            )}

            {/* Installing (app relaunches automatically) */}
            {updatePhase === "installing" && (
              <div className="flex items-center justify-center text-[11px] text-gray-500 gap-1.5">
                <Loader2 size={12} className="animate-spin" />
                {t("app.installing")}
              </div>
            )}

            {/* Network unreachable → configure proxy or continue anyway */}
            {updatePhase === "failed" && !updateError && networkResult && (
              <div>
                <p className="text-xs text-red-500 mb-1">{t("app.networkFail")}</p>
                <p className="text-[11px] text-gray-400 mb-4">
                  {networkResult.proxy_configured
                    ? t("app.networkProxyFailed")
                    : t("app.networkNoProxy")}
                </p>
                <div className="flex items-center justify-center gap-2">
                  <button
                    className={`px-4 py-2 text-xs rounded-lg ${APP_COLOR.btn} text-white font-medium ${APP_COLOR.btnHover} transition-colors`}
                    onClick={handleGoProxy}
                  >
                    {t("app.goProxy")}
                  </button>
                  <button
                    className="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-500 hover:bg-gray-50 transition-colors"
                    onClick={handleContinueDownload}
                  >
                    {t("app.retryDownload")}
                  </button>
                </div>
              </div>
            )}

            {/* Download/install failed → close and go back to the main modal */}
            {updatePhase === "failed" && updateError && (
              <div>
                <p className="text-xs text-red-500 mb-4 break-words whitespace-pre-wrap">
                  {updateError}
                </p>
                <div className="flex items-center justify-center gap-2">
                  <button
                    className="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-500 hover:bg-gray-50 transition-colors"
                    onClick={handleStopUpdate}
                  >
                    {t("common.close")}
                  </button>
                </div>
              </div>
            )}

            {/* Download timeout → manual download from GitHub or stop */}
            {updatePhase === "timeout" && (
              <div>
                <p className="text-xs text-amber-600 mb-1">{t("app.timeoutMsg")}</p>
                <p className="text-[11px] text-gray-400 mb-4">
                  {t("app.timeoutHint", { pct: updatePercent })}
                </p>
                <div className="flex items-center justify-center gap-2">
                  <a
                    href="https://github.com/MuZiCul/XDownload/releases"
                    target="_blank"
                    rel="noopener noreferrer"
                    className={`px-4 py-2 text-xs rounded-lg ${APP_COLOR.btn} text-white font-medium ${APP_COLOR.btnHover} transition-colors inline-flex items-center gap-1`}
                  >
                    {t("app.goGithub")}
                    <ArrowUpRight size={11} />
                  </a>
                  <button
                    className="px-3 py-2 text-xs rounded-lg border border-gray-200 text-gray-500 hover:bg-gray-50 transition-colors"
                    onClick={handleStopUpdate}
                  >
                    {t("app.stopUpdate")}
                  </button>
                </div>
              </div>
            )}
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

      {/* 退出确认弹窗（simple=无任务/队列持久化已开；完整=有任务且未自动保存）。
          托盘退出不提供「最小化到托盘」（托盘退出意图即真退出）。 */}
      {quitDialog && (
        <div className="dialog-overlay">
          <div className="dialog-content" onClick={(e) => e.stopPropagation()}>
            <h3 className="text-sm font-semibold text-zinc-900 mb-2">
              {t("quit.title")}
            </h3>
            <p className="text-xs text-zinc-500 mb-4 leading-relaxed">
              {quitDialog.simple ? t("quit.bodySimple") : t("quit.body")}
            </p>
            <div className="flex flex-col gap-1.5">
              {quitDialog.simple ? (
                <>
                  {/* 精简模式：无任务 / 队列持久化已开（任务自动保存）——
                      不询问保存进度。close/settings：最小化+退出；tray：退出。 */}
                  {quitDialog.source !== "tray" && (
                    <button
                      className="btn w-full text-sm flex items-center justify-center gap-2 py-2.5"
                      onClick={doHideToTray}
                    >
                      <Minimize2 size={15} />
                      {t("quit.minimizeToTray")}
                    </button>
                  )}
                  <button
                    className="btn btn-primary w-full text-sm flex items-center justify-center gap-2 py-2.5"
                    onClick={() => doQuit(false)}
                  >
                    <Power size={15} />
                    {t("quit.exit")}
                  </button>
                  {quitDialog.source === "tray" && (
                    <button
                      className="btn w-full text-sm py-2.5"
                      onClick={() => setQuitDialog(null)}
                    >
                      {t("common.cancel")}
                    </button>
                  )}
                </>
              ) : (
                <>
                  {/* 完整模式：有任务且队列持久化关闭 → 由用户决定是否保存进度。 */}
                  <button
                    className="btn btn-primary w-full text-sm flex items-center justify-center gap-2 py-2.5"
                    onClick={() => doQuit(true)}
                  >
                    <Save size={15} />
                    {t("quit.saveAndExit")}
                  </button>
                  <button
                    className="btn w-full text-sm flex items-center justify-center gap-2 py-2.5"
                    onClick={() => doQuit(false)}
                  >
                    <Power size={15} />
                    {t("quit.exitWithoutSave")}
                  </button>
                  {quitDialog.source !== "tray" && (
                    <button
                      className="btn w-full text-sm flex items-center justify-center gap-2 py-2.5"
                      onClick={doHideToTray}
                    >
                      <Minimize2 size={15} />
                      {t("quit.minimizeToTray")}
                    </button>
                  )}
                  <button
                    className="btn w-full text-sm py-2.5"
                    onClick={() => {
                      setQuitDialog(null);
                      // X 场景取消 → 隐藏到托盘；托盘/设置取消 → 无动作。
                      if (quitDialog.source === "close") {
                        getCurrentWindow().hide().catch(() => {});
                      }
                    }}
                  >
                    {t("common.cancel")}
                  </button>
                </>
              )}
            </div>
          </div>
        </div>
      )}

      {/* 深链已下载确认弹窗：浏览器扩展发来的链接已下载过，交用户逐条选择
          重新下载/取消；全部处理完（列表清空）后自动卸载。 */}
      {deepDups.length > 0 && (
        <DuplicateDownloadModal
          items={deepDups}
          onRedownload={handleDeepDupRedownload}
          onCancel={handleDeepDupCancel}
        />
      )}
    </QueryClientProvider>
  );
}

export default App;
