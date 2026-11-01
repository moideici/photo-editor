import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Doc, Layer, Operation } from "./types";
type Plugin = {
  path: string; caption: string; version: string; author: string; comment: string;
  pluginType: number; flags: number; compatibility: number; glyph: string | null;
};
type Catalog = { plugins: Plugin[]; errors: { path: string; message: string }[] };
type Outcome = { changed: boolean; newDocument: boolean; callbacks: string[]; message: string };
const kinds = ["Filtre", "Image", "Import", "Export", "Outil"];
export default function PflPanel({ layer, doc, busy, action }: {
  layer?: Layer; doc: Doc; busy: boolean; action: (op: Operation) => Promise<Doc | undefined>;
}) {
  const [catalog, setCatalog] = useState<Catalog>({ plugins: [], errors: [] });
  const [file, setFile] = useState(""), [path, setPath] = useState("");
  const [kind, setKind] = useState<number | null>(null);
  const [seconds, setSeconds] = useState(120), [loading, setLoading] = useState(false);
  const [error, setError] = useState(""), [message, setMessage] = useState("");
  const plugin = catalog.plugins.find(p => p.path === path);
  const disabled = busy || loading;
  useEffect(() => {
    if (isTauri()) void invoke<Catalog>("pfl_examples").then(c => {
      setCatalog(c); setPath(c.plugins[0]?.path ?? "");
    }).catch(e => setError(String(e)));
    const chooseKind = (event: Event) => {
      const selected = (event as CustomEvent<number>).detail;
      setKind(selected); setPath("");
    };
    window.addEventListener("prisme:pfl-type", chooseKind);
    return () => window.removeEventListener("prisme:pfl-type", chooseKind);
  }, []);
  async function work(run: () => Promise<void>) {
    setLoading(true); setError(""); setMessage("");
    try { await run(); } catch (e) { setError(String(e)); } finally { setLoading(false); }
  }
  async function load(value: string) {
    const p = await invoke<Plugin>("pfl_file", { path: value });
    setCatalog(c => ({ ...c, plugins: [...c.plugins.filter(k => k.path !== p.path), p] }));
    setPath(p.path); setFile(value); setKind(null);
  }
  return <details className="panel pfl-panel">
    <summary>Plugins PhotoFiltre PFL</summary>
    <div className="plugins-content">
      <div className="button-row">
        <button disabled={disabled} onClick={() => void work(async () => {
          const path = await open({ multiple: false, title: "Charger un plugin PhotoFiltre",
            filters: [{ name: "Plugins PhotoFiltre", extensions: ["pfl"] }] });
          if (typeof path === "string") await load(path);
        })}>Charger un PFL…</button>
        <button disabled={disabled} onClick={() => void work(async () => {
          const directory = await open({ directory: true, multiple: false, title: "Dossier de plugins PhotoFiltre" });
          if (typeof directory !== "string") return;
          const c = await invoke<Catalog>("pfl_scan", { directory });
          setCatalog(c); setPath(c.plugins[0]?.path ?? ""); setKind(null);
        })}>Dossier PFL…</button>
      </div>
      <label>Fichier PFL<input aria-label="Fichier PFL" value={file} disabled={disabled} onChange={e => setFile(e.target.value)} /></label>
      <button disabled={disabled || !file.trim()} onClick={() => void work(() => load(file.trim()))}>Charger ce PFL</button>
      <label>Catégorie
        <select aria-label="Catégorie PFL" disabled={disabled} value={kind ?? ""} onChange={e => {
          setKind(e.target.value === "" ? null : Number(e.target.value)); setPath("");
        }}>
          <option value="">Toutes les catégories</option>
          {kinds.map((name, i) => <option value={i} key={name}>{name}</option>)}
        </select>
      </label>
      <label>Plugin PhotoFiltre
        <select aria-label="Plugin PhotoFiltre actif" value={path} disabled={disabled} onChange={e => { setPath(e.target.value); setMessage(""); }}>
          <option value="">Choisir un plugin</option>
          {catalog.plugins.filter(p => kind === null || kind === p.pluginType).map(p =>
            <option value={p.path} key={p.path}>{kinds[p.pluginType]} / {p.caption}</option>)}
        </select>
      </label>
      {plugin && <>
        <p className="hint">{plugin.glyph && <img src={plugin.glyph} width={19} height={19} alt="" />} {plugin.caption} · {plugin.version} · {plugin.author}</p>
        <p className="hint">{plugin.comment}</p>
        <p className="hint">Compatibilité expérimentale PhotoFiltre x64, niveau {plugin.compatibility}. Les plugins Delphi utilisant leurs propres objets VCL peuvent être incompatibles.</p>
        <label>Délai maximal (secondes)
          <input type="number" aria-label="Délai PFL" min={0.1} max={600} step={1} value={seconds} disabled={disabled} onChange={e => setSeconds(+e.target.value)} />
        </label>
        <div className="button-row">
          <button disabled={disabled || !!doc.animation || !Number.isFinite(seconds) || seconds < .1 || seconds > 600
            || ([0, 1, 3].includes(plugin.pluginType) && !doc.open)
            || (plugin.pluginType === 0 && layer?.kind !== "raster")}
            onClick={() => void work(async () => {
              const result = await action({ type: "pfl", id: layer?.id ?? null, path: plugin.path, timeoutMs: Math.round(seconds * 1000) });
              if (result) {
                const outcome = await invoke<Outcome | null>("pfl_last_run");
                setMessage(outcome?.changed ? "Plugin PFL appliqué." : "Plugin PFL terminé.");
              }
            })}>Exécuter le plugin PFL</button>
          <button disabled={disabled} onClick={() => void work(async () => {
            setMessage(await invoke<string>("pfl_about", { path: plugin.path }));
          })}>À propos du PFL…</button>
        </div>
        <p className="hint">Les réglages sont ceux du plugin. Le rejeu par action n’est pas disponible pour ces extensions.</p>
      </>}
      {loading && <p className="hint" role="status">Opération PFL en cours…</p>}
      {message && <p className="hint" role="status">{message}</p>}
      {error && <p className="plugin-error" role="alert">{error}</p>}
      {catalog.errors.map(e => <p className="hint" key={e.path}>{e.path} : {e.message}</p>)}
    </div>
  </details>;
}

