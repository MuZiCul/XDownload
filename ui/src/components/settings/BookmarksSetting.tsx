import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { listBookmarks, loadSettings, getCookiesUsername } from "../../lib/bindings";
import {
  enqueueDownloadGlobal,
  buildBatchConfig,
} from "../../lib/downloadStore";
import type { BookmarksListItem } from "../../lib/types";
import { TaskSource } from "../../lib/types";
import { toast } from "sonner";
import { RefreshCw, Download, History, X, UserRound } from "lucide-react";
import { useI18n } from "../../lib/i18n";
import { runBookmarkSync, useBookmarkSync } from "../../lib/bookmarkSync";
import SectionTitle from "./SectionTitle";

/**
 * 书签管理设置卡片：手动同步 + 查看已同步书签目录。
 *
 * 同步流程：点「同步书签」→ 交给全局 `bookmarkSync` store，由 App 根部的
 * `BookmarkSyncModal` 显示同步进程（无法关闭）→ 完成后进入预览确认 →
 * 确认后才入队。全局状态跨 tab 保留，切 tab 不会丢失「同步中」。
 *
 * 查看流程：点「查看书签」→ 从本地 bookmarks 表读出全部同步过的书签
 * （含视频与无视频）→ 拟态窗展示下载状态，可单独下载/重新下载。
 */
export default function BookmarksSetting() {
  const { t } = useI18n();
  const { phase } = useBookmarkSync();
  const syncing = phase === "syncing";
  // 已同步书签目录弹窗。
  const [listOpen, setListOpen] = useState(false);
  const [listLoading, setListLoading] = useState(false);
  const [bookmarks, setBookmarks] = useState<BookmarksListItem[]>([]);
  // 当前 cookie 来源对应的 x.com 用户名（静默获取，失败为 null 不阻塞 UI）。
  const [cookiesUsername, setCookiesUsername] = useState<string | null>(null);
  // 手动「获取用户」进行中（后端会重试 3 次、间隔 5s）。
  const [gettingUser, setGettingUser] = useState(false);
  // 后端 verify 全失败时固化的占位用户名（表示当前 cookies 无有效用户）。
  const INVALID_USER = "@None";

  // 挂载 + config-applied（cookies 来源可能变化）时静默获取用户名。
  useEffect(() => {
    let cancelled = false;
    getCookiesUsername().then((name) => {
      if (!cancelled) setCookiesUsername(name);
    }).catch(() => {});
    const onConfigApplied = () => {
      getCookiesUsername().then((name) => {
        if (!cancelled) setCookiesUsername(name);
      }).catch(() => {});
    };
    window.addEventListener("config-applied", onConfigApplied);
    return () => {
      cancelled = true;
      window.removeEventListener("config-applied", onConfigApplied);
    };
  }, []);

  // 手动获取用户名：force=true 强制 dump+verify（忽略缓存，感知 cookies 内容变化）；
  // 后端首次失败会自动重试 3 次（间隔 5s）。成功更新 state（解锁同步/查看按钮）；失败保持禁用。
  const handleGetUser = async () => {
    setGettingUser(true);
    try {
      const name = await getCookiesUsername(true);
      if (name && name !== INVALID_USER) {
        setCookiesUsername(name);
        toast.success(t("bookmarks.userFetched", { username: name }));
      } else {
        setCookiesUsername(INVALID_USER);
        toast.error(t("bookmarks.userFetchFail"));
      }
    } catch (err: any) {
      toast.error(String(err));
    } finally {
      setGettingUser(false);
    }
  };

  const loadList = async () => {
    setListLoading(true);
    try {
      setBookmarks(await listBookmarks());
    } catch (err: any) {
      toast.error(String(err));
    } finally {
      setListLoading(false);
    }
  };

  const openList = () => {
    setListOpen(true);
    loadList();
  };

  const closeList = () => setListOpen(false);

  // 「查看书签」弹窗打开期间实时刷新下载状态：书签视频下载完成后后端历史
  // 已写入，但弹窗打开时是一次快照——监听 download-finished 自动 reload，
  // 避免用户要等重启/重开弹窗才能看到「已下载」标记。
  useEffect(() => {
    if (!listOpen) return;
    let unlisten: UnlistenFn | null = null;
    let cancelled = false;
    (async () => {
      try {
        const un = await listen("download-finished", () => {
          if (!cancelled) loadList();
        });
        // listen 是异步的：若弹窗已在注册完成前被关闭，立即注销避免监听器泄漏。
        if (cancelled) {
          un();
          return;
        }
        unlisten = un;
      } catch {
        // 监听失败不阻塞弹窗使用（下次打开/手动重进仍会刷新）。
      }
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [listOpen]);

  /** 从书签目录下载/重新下载一个视频（复用批量下载的配置与入队逻辑）。 */
  const handleListDownload = async (item: BookmarksListItem) => {
    try {
      const s = await loadSettings().catch(() => null);
      await enqueueDownloadGlobal(
        buildBatchConfig(item.url, item.video_id, s),
        { autoStart: true, source: TaskSource.Bookmark }
      );
      toast.success(t("bookmarks.enqueued", { count: 1 }));
      // 刷新目录里的下载状态。
      loadList();
    } catch (err: any) {
      toast.error(t("bookmarks.enqueueFail", { err }));
    }
  };

  return (
    <div className="section-card">
      <SectionTitle
        title={
          <>
            {t("bookmarks.title")}
            {cookiesUsername && cookiesUsername !== INVALID_USER && (
              <span className="normal-case font-normal text-[10px] text-green-600 ml-2">
                ● {t("bookmarks.statusLoggedIn", { user: cookiesUsername })}
              </span>
            )}
            {cookiesUsername === INVALID_USER && (
              <span className="normal-case font-normal text-[10px] text-red-500 ml-2">
                ● {t("bookmarks.statusNoUser")}
              </span>
            )}
          </>
        }
        tip={t("bookmarks.hint")}
      />
      <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
        <button
          className="btn flex items-center gap-1"
          onClick={handleGetUser}
          disabled={gettingUser || syncing}
        >
          <UserRound size={13} className={gettingUser ? "animate-spin" : ""} />
          {gettingUser
            ? t("bookmarks.gettingUser")
            : cookiesUsername && cookiesUsername !== INVALID_USER
              ? t("bookmarks.reloadUser")
              : t("bookmarks.getUser")}
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={runBookmarkSync}
          disabled={!cookiesUsername || cookiesUsername === INVALID_USER || syncing}
        >
          <RefreshCw size={13} className={syncing ? "animate-spin" : ""} />
          {syncing ? t("bookmarks.syncing") : t("bookmarks.sync")}
        </button>
        <button
          className="btn flex items-center gap-1"
          onClick={openList}
          disabled={!cookiesUsername || cookiesUsername === INVALID_USER || syncing}
        >
          <History size={13} />
          {t("bookmarks.viewList")}
        </button>
      </div>
      {/* 已同步书签目录拟态窗：从本地 bookmarks 表读取，展示下载状态，
          可单独下载 / 重新下载（复用批量下载逻辑）。 */}
      {listOpen && (
        <div className="dialog-overlay" onClick={closeList}>
          <div
            className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 z-[60] w-[520px] max-w-[92vw] bg-white/80 backdrop-blur-xl rounded-2xl shadow-2xl border border-white/50 p-5"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-semibold text-zinc-900">
                {t("bookmarks.listTitle")}
              </span>
              <button
                className="text-zinc-400 hover:text-zinc-600 transition-colors"
                onClick={closeList}
              >
                <X size={16} />
              </button>
            </div>
            <p className="text-xs text-zinc-500 mb-3 leading-relaxed">
              {t("bookmarks.listHint")}
            </p>
            {listLoading ? (
              <div className="py-8 text-center text-xs text-zinc-400">
                {t("bookmarks.listLoading")}
              </div>
            ) : bookmarks.length === 0 ? (
              <div className="py-8 text-center text-xs text-zinc-400">
                {t("bookmarks.listEmpty")}
              </div>
            ) : (
              <div className="max-h-[300px] overflow-y-auto space-y-1.5 mb-4 pr-1">
                {bookmarks.map((item) => (
                  <div
                    key={item.id}
                    className={`flex items-start gap-2.5 px-3 py-2 rounded-lg border transition-colors ${
                      item.has_video
                        ? item.downloaded
                          ? "bg-zinc-100/80 border-zinc-200/70"
                          : "bg-blue-50/60 border-blue-400/60"
                        : "bg-zinc-50/80 border-zinc-200/50"
                    }`}
                  >
                    <a
                      href={item.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="block flex-1 min-w-0"
                    >
                      <div className="text-xs font-medium text-zinc-800 line-clamp-2">
                        {item.title || t("bookmarks.noTitle")}
                      </div>
                      <div className="text-[11px] text-zinc-500 mt-0.5">
                        @{item.handle} · {item.author_name}
                      </div>
                    </a>
                    <div className="flex items-center gap-1.5 shrink-0 mt-0.5">
                      {item.has_video ? (
                        item.downloaded ? (
                          <>
                            <span className="text-[10px] text-zinc-500 bg-zinc-200/80 rounded px-1.5 py-0.5">
                              {t("bookmarks.downloaded")}
                            </span>
                            <button
                              className="p-1 rounded hover:bg-zinc-100 text-emerald-700"
                              onClick={() => handleListDownload(item)}
                              title={t("video.redownload")}
                            >
                              <RefreshCw size={13} />
                            </button>
                          </>
                        ) : (
                          <button
                            className="p-1.5 rounded hover:bg-blue-50 text-blue-600 flex items-center gap-1 text-[11px]"
                            onClick={() => handleListDownload(item)}
                          >
                            <Download size={12} />
                            {t("bookmarks.download")}
                          </button>
                        )
                      ) : (
                        <span className="text-[10px] text-zinc-400 bg-zinc-100 border border-zinc-200 rounded px-1.5 py-0.5">
                          {t("bookmarks.noVideoTag")}
                        </span>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
            <div className="flex justify-end">
              <button className="btn" onClick={closeList}>
                {t("bookmarks.cancel")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
