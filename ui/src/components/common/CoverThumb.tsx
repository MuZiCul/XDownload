import { useState } from "react";
import appIcon from "../../assets/icon.png";

type Props = {
  src: string | null;
  /** 任务面板卡片用：横图 object-cover 上下贴满、竖图高度贴边 + 左右模糊填充。 */
  stretch?: boolean;
  /** stretch 模式下覆盖容器尺寸（默认 w-40 h-[90px]）。 */
  boxClass?: string;
  /** 隐私模式：封面用毛玻璃覆盖。 */
  blurred?: boolean;
};

/** Cover thumbnail that falls back to the app icon when the URL is missing or
 *  fails to load. */
export default function CoverThumb({ src, stretch, boxClass, blurred }: Props) {
  const [failed, setFailed] = useState(false);
  // 图片实际宽高比是否横图（宽 > 高）。null = 未知（未加载完成）。
  const [isLandscape, setIsLandscape] = useState<boolean | null>(null);
  const showFallback = !src || failed;

  const handleLoad = (e: React.SyntheticEvent<HTMLImageElement>) => {
    const el = e.currentTarget;
    const w = el.naturalWidth;
    const h = el.naturalHeight;
    if (w > 0 && h > 0) setIsLandscape(w > h);
  };

  return (
    <div
      className={
        stretch
          ? `relative ${boxClass ?? "w-40 h-[90px]"} rounded-lg border border-zinc-200 bg-zinc-900 overflow-hidden shrink-0 flex items-center justify-center`
          : "w-28 h-[72px] rounded-lg border border-zinc-200 bg-zinc-900 overflow-hidden shrink-0 flex items-center justify-center"
      }
    >
      {showFallback ? (
        <img src={appIcon} alt="app" className="w-12 h-12 object-contain opacity-90" />
      ) : stretch ? (
        <>
          {/* 模糊背景：填满容器，非横图（竖/方/未知）剩余空白由此垫底（无黑边） */}
          {isLandscape !== true && (
            <img
              src={src}
              alt=""
              className="absolute inset-0 w-full h-full object-cover blur-md scale-110"
            />
          )}
          {/* 主图：横图 cover 填满（上下贴满、左右裁切）；竖图 contain（高度贴边、左右模糊填充） */}
          <img
            src={src}
            alt="thumbnail"
            onError={() => setFailed(true)}
            onLoad={handleLoad}
            className={
              isLandscape
                ? "relative w-full h-full object-cover"
                : "relative w-full h-full object-contain"
            }
          />
        </>
      ) : (
        <img
          src={src}
          alt="thumbnail"
          onError={() => setFailed(true)}
          className="w-full h-full object-cover"
        />
      )}
      {blurred && (
        <div className="absolute inset-0 z-10 backdrop-blur-md bg-white/20" />
      )}
    </div>
  );
}
