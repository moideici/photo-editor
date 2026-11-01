import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { Layer, Operation, Doc } from "./types";
type Parameter = {
  key: string;
  label: string;
  min: number;
  max: number;
  default: number;
  step: number;
};
type Plugin = {
  path: string;
  schemaVersion: number;
  id: string;
  name: string;
  version: string;
  author: string;
  description: string;
  kind: "native" | "rhai";
  parameters: Parameter[];
  timeoutMs: number;
};
type Catalog = {
  plugins: Plugin[];
  errors: { path: string; message: string }[];
};
export default function PluginsPanel({
  layer,
  doc,
  busy,
  action,
}: {
  layer?: Layer;
  doc: Doc;
  busy: boolean;
  action: (op: Operation) => Promise<Doc | undefined>;
}) {
  const [catalog, setCatalog] = useState<Catalog>({ plugins: [], errors: [] }),
    [path, setPath] = useState(""),
    [params, setParams] = useState<Record<string, number>>({}),
    [loading, setLoading] = useState(false),
    [manifestPath, setManifestPath] = useState(""),
    [error, setError] = useState("");
  const plugin = catalog.plugins.find((p) => p.path === path);
  function choose(p: Plugin) {
    setPath(p.path);
    setParams(Object.fromEntries(p.parameters.map((k) => [k.key, k.default])));
  }
  useEffect(() => {
    if (!isTauri()) return;
    void invoke<Catalog>("plugin_examples")
      .then((c) => {
        setCatalog(c);
        if (c.plugins[0]) choose(c.plugins[0]);
      })
      .catch((e) => setError(String(e)));
  }, []);
  async function scan() {
    setLoading(true);
    try {
      const directory = await open({
        directory: true,
        multiple: false,
        title: "Dossier de plugins Prisme",
      });
      if (typeof directory !== "string") return;
      const c = await invoke<Catalog>("plugins_scan", { directory });
      setCatalog(c);
      if (c.plugins[0]) choose(c.plugins[0]);
      else setPath("");
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }
  async function load() {
    setLoading(true);
    try {
      const path = await open({
        multiple: false,
        title: "Charger un plugin Prisme",
        filters: [{ name: "Manifeste de plugin Prisme", extensions: ["json"] }],
      });
      if (typeof path !== "string") return;
      await loadManifest(path);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }
  async function loadManifest(path: string) {
    setLoading(true);
    try {
      const p = await invoke<Plugin>("plugin_manifest", { path });
      setCatalog((c) => ({
        ...c,
        plugins: [...c.plugins.filter((k) => k.path !== p.path), p],
      }));
      choose(p);
      setManifestPath(path);
      setError("");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }
  return (
    <details className="panel plugins-panel">
      <summary>Plugins Prisme</summary>
      <div className="plugins-content">
        <div className="button-row">
          <button onClick={() => void load()} disabled={busy || loading}>
            Charger un manifeste…
          </button>
          <button onClick={() => void scan()} disabled={busy || loading}>
            Dossier…
          </button>
        </div>
        <label>
          Fichier du plugin
          <input
            aria-label="Fichier du plugin"
            value={manifestPath}
            onChange={(e) => setManifestPath(e.target.value)}
            disabled={busy || loading}
          />
        </label>
        <button
          disabled={busy || loading || !manifestPath.trim()}
          onClick={() => void loadManifest(manifestPath.trim())}
        >
          Charger ce fichier
        </button>
        <label>
          Filtre{" "}
          <select
            aria-label="Plugin actif"
            disabled={busy || loading}
            value={path}
            onChange={(e) => {
              const p = catalog.plugins.find((p) => p.path === e.target.value);
              if (p) choose(p);
              else {
                setPath("");
                setParams({});
              }
            }}
          >
            <option value="">Choisir un plugin</option>
            {catalog.plugins.map((p) => (
              <option value={p.path} key={p.path}>
                {p.name}
              </option>
            ))}
          </select>
        </label>
        {plugin && (
          <>
            <p className="hint">
              {plugin.description} ·{" "}
              {plugin.kind === "native" ? "DLL native" : "Script Rhai"} ·{" "}
              {plugin.version}
              {plugin.author ? " · " + plugin.author : ""}
            </p>
            {plugin.parameters.map((p) => (
              <label key={p.key}>
                {p.label}
                <input
                  type="number"
                  aria-label={`Plugin ${p.label}`}
                  min={p.min}
                  max={p.max}
                  step={p.step}
                  value={params[p.key] ?? p.default}
                  onChange={(e) =>
                    setParams((old) => ({ ...old, [p.key]: +e.target.value }))
                  }
                />
              </label>
            ))}
            <button
              disabled={
                busy ||
                loading ||
                layer?.kind !== "raster" ||
                !!doc.animation ||
                !plugin.parameters.every(
                  (p) =>
                    Number.isFinite(params[p.key]) &&
                    params[p.key] >= p.min &&
                    params[p.key] <= p.max,
                )
              }
              onClick={() =>
                layer &&
                void action({
                  type: "plugin",
                  id: layer.id,
                  manifest: plugin.path,
                  params,
                })
              }
            >
              Appliquer le plugin
            </button>
          </>
        )}
        {error && (
          <p className="plugin-error" role="alert">
            {error}
          </p>
        )}
        {catalog.errors.map((e) => (
          <p className="hint" key={e.path}>
            {e.path}: {e.message}
          </p>
        ))}
      </div>
    </details>
  );
}
