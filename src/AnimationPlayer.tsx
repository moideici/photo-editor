import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { AnimationInfo } from "./types";
export default function AnimationPlayer({
  animation,
  stopToken,
  busy,
  onFrame,
  onSelect,
  onExtract,
  onError,
}: {
  animation: AnimationInfo;
  stopToken: number;
  busy: boolean;
  onFrame: (index: number, src: string) => void;
  onSelect: (index: number) => void;
  onExtract: (index: number) => void;
  onError: (message: string) => void;
}) {
  const [playing, setPlaying] = useState(false),
    [current, setCurrent] = useState(animation.current),
    [repeat, setRepeat] = useState<"file" | "forever" | "once">("file");
  const frame = useRef(animation.current),
    cache = useRef(new Map<number, string>()),
    callbacks = useRef({ onFrame, onSelect, onError });
  callbacks.current = { onFrame, onSelect, onError };
  useEffect(() => {
    setPlaying(false);
  }, [stopToken]);
  useEffect(() => {
    setPlaying(false);
    setCurrent(animation.current);
    frame.current = animation.current;
    cache.current.clear();
  }, [animation.id]);
  useEffect(() => {
    setCurrent(animation.current);
    frame.current = animation.current;
  }, [animation.current]);
  useEffect(() => {
    if (!playing) return;
    let cancelled = false,
      timer: ReturnType<typeof setTimeout> | undefined;
    const delays = animation.delays.map((v) => Math.max(10, v)),
      total = delays.reduce((a, b) => a + b, 0);
    const offset = delays.slice(0, frame.current).reduce((a, b) => a + b, 0),
      started = performance.now() - offset;
    const limit =
      repeat === "forever"
        ? Infinity
        : repeat === "once"
          ? 1
          : animation.loops === 0
            ? Infinity
            : animation.loops;
    const tick = async () => {
      if (cancelled) return;
      const elapsed = performance.now() - started;
      if (elapsed >= total * limit) {
        setPlaying(false);
        callbacks.current.onSelect(animation.frameCount - 1);
        return;
      }
      let time = elapsed % total,
        index = 0;
      while (index < delays.length - 1 && time >= delays[index]) {
        time -= delays[index];
        index++;
      }
      try {
        let src = cache.current.get(index);
        if (!src) {
          src = await invoke<string>("animation_frame", {
            id: animation.id,
            index,
          });
          cache.current.set(index, src);
          let bytes = [...cache.current.values()].reduce(
            (sum, s) => sum + s.length * 2,
            0,
          );
          while (bytes > 32 * 1024 * 1024 && cache.current.size > 1) {
            const oldest = cache.current.keys().next().value!;
            bytes -= cache.current.get(oldest)!.length * 2;
            cache.current.delete(oldest);
          }
        }
        if (cancelled) return;
        if (frame.current !== index || cache.current.size === 1) {
          frame.current = index;
          setCurrent(index);
          callbacks.current.onFrame(index, src);
        }
      } catch (e) {
        if (!cancelled) {
          setPlaying(false);
          callbacks.current.onError(String(e));
        }
        return;
      }
      if (!cancelled)
        timer = setTimeout(
          () => void tick(),
          Math.max(10, Math.min(50, delays[index] - time)),
        );
    };
    void tick();
    return () => {
      cancelled = true;
      if (timer) clearTimeout(timer);
    };
  }, [playing, animation.id, repeat]);
  const pause = () => {
    setPlaying(false);
    onSelect(frame.current);
  };
  return (
    <div className="animation-player" aria-label="Lecture GIF">
      <button
        type="button"
        disabled={busy}
        onClick={() => (playing ? pause() : setPlaying(true))}
      >
        {playing ? "Pause" : "Lire le GIF"}
      </button>
      <input
        aria-label="Image du GIF"
        type="range"
        min={0}
        max={animation.frameCount - 1}
        value={current}
        disabled={busy || playing}
        onChange={(e) => {
          const index = +e.target.value;
          frame.current = index;
          setCurrent(index);
          onSelect(index);
        }}
      />
      <span>
        Image {current + 1} / {animation.frameCount} ·{" "}
        {animation.delays[current]} ms
      </span>
      <select
        aria-label="Répétition du GIF"
        value={repeat}
        onChange={(e) => setRepeat(e.target.value as typeof repeat)}
      >
        <option value="file">Selon le fichier</option>
        <option value="forever">En boucle</option>
        <option value="once">Une fois</option>
      </select>
      <button
        type="button"
        disabled={busy}
        onClick={() => {
          setPlaying(false);
          onExtract(frame.current);
        }}
      >
        Éditer cette image
      </button>
    </div>
  );
}
