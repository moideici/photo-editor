import { useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { Doc, Layer, Operation } from "./types";

type Filter = {
  key: string; path: string; name: string; category: string;
  architectures: string[]; resourceId: number | null; error: string | null;
};
type Settings = {
  showUi: boolean; parameters: string | null; descriptor: unknown;
  timeoutMs: number; foreground: number[]; background: number[]; resolution: number;
};
type Catalog = { plugins: Filter[]; errors: { path: string; message: string }[] };
export default function EightBfPanel({ layer, doc, busy, action, color, secondary }: {
  layer?: Layer; doc: Doc; busy: boolean;
  color: string; secondary: string;
  action: (op: Operation) => Promise<Doc | undefined>;
}) {
  const [catalog, setCatalog] = useState<Catalog>({ plugins: [], errors: [] });
  const [file, setFile] = useState("");
  const [key, setKey] = useState("");
  const [settings, setSettings] = useState<Settings | null>(null);
  const [showUi, setShowUi] = useState(true);
  const [seconds, setSeconds] = useState(120);
  const [loading, setLoading] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const selection = useRef(0);
  const filter = catalog.plugins.find((p) => p.key === key);
  const disabled = busy || loading;
  async function choose(p?: Filter) {
    const revision = ++selection.current;
    setKey(p?.key ?? ""); setSettings(null); setMessage("");
    if (!p) return;
    const saved = await invoke<Settings | null>("eightbf_last_settings", { key: p.key });
    if (revision === selection.current) {
      setSettings(saved);
      setSeconds((saved?.timeoutMs ?? 120000) / 1000);
    }
  }
  async function work(run: () => Promise<void>) {
    setLoading(true); setError(""); setMessage("");
    try { await run(); } catch (e) { setError(String(e)); } finally { setLoading(false); }
  }
  async function load(path: string) {
    const plugins = await invoke<Filter[]>("eightbf_file", { path });
    setCatalog((c) => ({ ...c, plugins: [...c.plugins.filter((p) => p.path !== plugins[0]?.path), ...plugins] }));
    setFile(path);
    await choose(plugins.find((p) => !p.error) ?? plugins[0]);
  }
  async function apply() {
    if (!filter || !layer) return;
    if (!/^#[0-9a-f]{6}$/i.test(color) || !/^#[0-9a-f]{6}$/i.test(secondary)) {
      throw new Error("Validez les couleurs de premier plan et d’arrière-plan avant le filtre.");
    }
    const rgb = (hex: string) => [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16));
    const result = await action({
      type: "eightBf", id: layer.id, path: filter.path, key: filter.key,
      settings: { ...settings, showUi, timeoutMs: Math.round(seconds * 1000),
        foreground: rgb(color), background: rgb(secondary) },
    });
    if (result) {
      setSettings(await invoke<Settings | null>("eightbf_last_settings", { key: filter.key }));
      setMessage("Filtre appliqué. Ses paramètres sont conservés pour le rejeu.");
    }
  }
  return <details className="panel eightbf-panel">
    <summary>Filtres Photoshop 8BF</summary>
    <div className="plugins-content">
      <div className="button-row">
        <button disabled={disabled} onClick={() => void work(async () => {
          const path = await open({ multiple: false, title: "Charger un filtre Photoshop",
            filters: [{ name: "Filtres Photoshop", extensions: ["8bf", "eff"] }] });
          if (typeof path === "string") await load(path);
        })}>Charger un 8BF…</button>
        <button disabled={disabled} onClick={() => void work(async () => {
          const directory = await open({ directory: true, multiple: false, title: "Dossier de filtres Photoshop" });
          if (typeof directory !== "string") return;
          const c = await invoke<Catalog>("eightbf_scan", { directory });
          setCatalog(c); await choose(c.plugins.find((p) => !p.error) ?? c.plugins[0]);
        })}>Dossier 8BF…</button>
      </div>
      <label>Fichier 8BF
        <input aria-label="Fichier 8BF" value={file} disabled={disabled}
          onChange={(e) => setFile(e.target.value)} />
      </label>
      <button disabled={disabled || !file.trim()} onClick={() => void work(() => load(file.trim()))}>
        Charger ce 8BF
      </button>
      <label>Filtre Photoshop
        <select aria-label="Filtre Photoshop actif" value={key} disabled={disabled}
          onChange={(e) => void work(() => choose(catalog.plugins.find((p) => p.key === e.target.value)))}>
          <option value="">Choisir un filtre</option>
          {catalog.plugins.map((p) => <option key={p.key} value={p.key}>
            {p.category} / {p.name}{p.error ? " — incompatible" : ""}
          </option>)}
        </select>
      </label>
      {filter && <>
        <p className="hint">{filter.name} · {filter.architectures.join(", ")} · ressource {filter.resourceId ?? "sans numéro"}</p>
        {filter.error && <p role="alert" className="plugin-error">{filter.error}</p>}
        <label><input type="checkbox" aria-label="Afficher le dialogue du filtre" checked={showUi}
          disabled={disabled} onChange={(e) => setShowUi(e.target.checked)} />Afficher le dialogue du filtre</label>
        <label>Délai maximal (secondes)
          <input type="number" aria-label="Délai 8BF" min={0.1} max={600} step={1}
            value={seconds} disabled={disabled} onChange={(e) => setSeconds(+e.target.value)} />
        </label>
        <div className="button-row">
          <button disabled={disabled || !!filter.error || layer?.kind !== "raster" || !!doc.animation
            || !Number.isFinite(seconds) || seconds < 0.1 || seconds > 600}
            onClick={() => void work(apply)}>Appliquer le filtre 8BF</button>
          <button disabled={disabled || !!filter.error} onClick={() => void work(async () => {
            const text = await invoke<string>("eightbf_about", { path: filter.path, key: filter.key });
            setMessage(text || "Le dialogue À propos est fermé.");
          })}>À propos du 8BF…</button>
        </div>
        <div className="button-row">
          <button disabled={disabled || !settings || !!filter.error} onClick={() => void work(async () => {
            const destination = await save({ title: "Enregistrer les paramètres du filtre",
              defaultPath: filter.name + ".prisme-8bf-preset",
              filters: [{ name: "Préréglage 8BF Prisme", extensions: ["prisme-8bf-preset"] }] });
            if (!destination) return;
            await invoke("eightbf_save_preset", { path: filter.path, key: filter.key, destination });
            setMessage("Préréglage enregistré.");
          })}>Enregistrer le préréglage…</button>
          <button disabled={disabled || !!filter.error} onClick={() => void work(async () => {
            const source = await open({ multiple: false, title: "Charger les paramètres du filtre",
              filters: [{ name: "Préréglage 8BF Prisme", extensions: ["prisme-8bf-preset"] }] });
            if (typeof source !== "string") return;
            const saved = await invoke<Settings>("eightbf_load_preset", { path: filter.path, key: filter.key, source });
            setSettings(saved); setSeconds(saved.timeoutMs / 1000); setShowUi(false);
            setMessage("Préréglage chargé. Le prochain appel utilisera ces paramètres.");
          })}>Charger un préréglage…</button>
          <button disabled={disabled || !settings} onClick={() => {
            setSettings(null); setShowUi(true); setSeconds(120); setMessage("Valeurs initiales au prochain appel.");
          }}>Réinitialiser</button>
        </div>
        <p className="hint">Les actions conservent les paramètres renvoyés par le filtre et le rejouent sans dialogue.
          Compatibilité selon les suites demandées par chaque filtre ; exécution dans un processus séparé.</p>
      </>}
      {loading && <p role="status" className="hint">Opération 8BF en cours…</p>}
      {message && <p role="status" className="hint">{message}</p>}
      {error && <p role="alert" className="plugin-error">{error}</p>}
      {catalog.errors.map((e) => <p className="hint" key={e.path}>{e.path} : {e.message}</p>)}
    </div>
  </details>;
}

