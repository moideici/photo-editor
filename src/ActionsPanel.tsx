import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { Doc, Operation } from "./types";
type RecordedStep = { operation: Operation; layerFromTop: number | null };
type Recorder = {
  recording: boolean;
  action: { version: number; name: string; steps: RecordedStep[] };
};
type Batch = {
  id: number;
  running: boolean;
  cancelled: boolean;
  total: number;
  current: string;
  results: { source: string; output: string | null; error: string | null }[];
};
const initial: Recorder = {
  recording: false,
  action: { version: 1, name: "Nouvelle action", steps: [] },
};
const names: Record<string, string> = {
  filter: "Filtre",
  plugin: "Plugin",
  resize: "Redimensionner",
  rotate: "Rotation",
  flip: "Miroir",
  canvas: "Zone de travail",
  crop: "Recadrage",
  effects: "Effets",
  asset: "Ressource",
  convertIndexed: "Couleurs indexées",
  convertRgb: "RVB",
  paletteColor: "Palette",
  transparency: "Transparence",
  stroke: "Trait",
  retouch: "Retouche",
  fill: "Remplissage",
  gradient: "Dégradé",
  selection: "Sélection",
  selectColor: "Sélection couleur",
  modifySelection: "Modifier la sélection",
  invertSelection: "Inverser la sélection",
  rememberSelection: "Mémoriser",
  restoreSelection: "Rappeler la sélection",
  forgetSelection: "Oublier la sélection",
  addLayer: "Ajouter un calque",
  deleteLayer: "Supprimer un calque",
  duplicateLayer: "Dupliquer le calque",
  layer: "Propriétés du calque",
  reorder: "Ordre des calques",
  mergeDown: "Fusionner",
  flatten: "Aplatir",
  rasterize: "Rastériser",
  shape: "Nouvelle forme",
};
const basename = (p: string) => p.split(/[\\/]/).at(-1) ?? p;
export default function ActionsPanel({
  doc,
  busy,
  onReplay,
  onError,
}: {
  doc: Doc;
  busy: boolean;
  onReplay: () => void;
  onError: (e: string) => void;
}) {
  const [rec, setRec] = useState(initial),
    [name, setName] = useState(initial.action.name),
    [working, setWorking] = useState(false),
    [batch, setBatch] = useState<Batch | null>(null);
  const [paths, setPaths] = useState<string[]>([]),
    [directory, setDirectory] = useState(""),
    [format, setFormat] = useState("png"),
    [quality, setQuality] = useState(90);
  useEffect(() => {
    if (!isTauri()) return;
    let cancelled = false;
    void invoke<Recorder>("automation_state")
      .then((v) => {
        if (!cancelled) {
          setRec(v);
          setName(v.action.name);
        }
      })
      .catch((e) => onError(String(e)));
    return () => {
      cancelled = true;
    };
  }, [doc]);
  useEffect(() => {
    if (!isTauri()) return;
    void invoke<Batch>("batch_status")
      .then(setBatch)
      .catch((e) => onError(String(e)));
  }, []);
  useEffect(() => {
    if (!batch?.running) return;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout>;
    const poll = async () => {
      try {
        const state = await invoke<Batch>("batch_status");
        if (cancelled) return;
        setBatch(state);
        if (state.running) timer = setTimeout(() => void poll(), 500);
      } catch (e) {
        if (!cancelled) onError(String(e));
      }
    };
    timer = setTimeout(() => void poll(), 500);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [batch?.running, batch?.id]);
  async function control(control: Record<string, unknown>) {
    setWorking(true);
    try {
      const value = await invoke<Recorder>("automation_control", { control });
      setRec(value);
      setName(value.action.name);
    } catch (e) {
      onError(String(e));
    } finally {
      setWorking(false);
    }
  }
  async function load() {
    try {
      const path = await open({
        title: "Charger une action Prisme",
        filters: [
          { name: "Action Prisme", extensions: ["prisme-action", "json"] },
        ],
      });
      if (typeof path === "string") await control({ mode: "load", path });
    } catch (e) {
      onError(String(e));
    }
  }
  async function store() {
    try {
      const path = await save({
        title: "Enregistrer une action Prisme",
        defaultPath: `${name || "action"}.prisme-action`,
        filters: [{ name: "Action Prisme", extensions: ["prisme-action"] }],
      });
      if (path) {
        await control({ mode: "rename", name });
        await control({ mode: "save", path });
      }
    } catch (e) {
      onError(String(e));
    }
  }
  async function files() {
    try {
      const result = await open({ title: "Images à traiter", multiple: true });
      if (result) setPaths(Array.isArray(result) ? result : [result]);
    } catch (e) {
      onError(String(e));
    }
  }
  async function folder() {
    try {
      const result = await open({
        title: "Dossier de sortie du lot",
        directory: true,
      });
      if (typeof result === "string") setDirectory(result);
    } catch (e) {
      onError(String(e));
    }
  }
  async function startBatch() {
    setWorking(true);
    try {
      setBatch(
        await invoke<Batch>("batch_start", {
          request: { paths, directory, format, quality },
        }),
      );
    } catch (e) {
      onError(String(e));
    } finally {
      setWorking(false);
    }
  }
  const disabled = busy || working;
  return (
    <details className="panel actions-panel">
      <summary>
        Actions et traitement par lot {rec.recording && <b>● Enregistrement</b>}
      </summary>
      <label>
        Nom de l’action
        <input
          aria-label="Nom de l’action"
          maxLength={128}
          value={name}
          disabled={disabled}
          onChange={(e) => setName(e.target.value)}
          onBlur={() => {
            if (name !== rec.action.name)
              void control({ mode: "rename", name });
          }}
        />
      </label>
      <div className="action-buttons">
        <button
          type="button"
          disabled={
            disabled || (!rec.recording && (!doc.open || !!doc.animation))
          }
          onClick={() =>
            void control(
              rec.recording ? { mode: "stop" } : { mode: "start", name },
            )
          }
        >
          {rec.recording ? "Arrêter l’enregistrement" : "Nouvel enregistrement"}
        </button>
        <button
          type="button"
          disabled={
            disabled || rec.recording || !rec.action.steps.length || !doc.open
          }
          onClick={onReplay}
        >
          Rejouer l’action
        </button>
      </div>
      <p className="hint">
        {rec.recording
          ? "Les modifications sont enregistrées. Arrêtez avant d’annuler ou d’ouvrir un autre fichier."
          : "Rejouer crée une seule étape annulable. Les coordonnées et dimensions sont absolues ; les calques sont repérés depuis le haut."}
      </p>
      <ol className="action-steps">
        {rec.action.steps.map((step, i) => (
          <li key={i}>
            <span>
              {names[step.operation.type] ?? step.operation.type}
              {step.operation.type === "filter"
                ? ` · ${step.operation.name}`
                : ""}
              {step.layerFromTop !== null
                ? ` · calque ${step.layerFromTop + 1}`
                : ""}
            </span>
            <button
              type="button"
              aria-label={`Retirer l’étape ${i + 1}`}
              disabled={disabled || rec.recording}
              onClick={() => void control({ mode: "remove", index: i })}
            >
              ×
            </button>
          </li>
        ))}
      </ol>
      <div className="action-buttons">
        <button
          type="button"
          disabled={disabled || rec.recording}
          onClick={() => void load()}
        >
          Charger une action
        </button>
        <button
          type="button"
          disabled={disabled || !rec.action.steps.length}
          onClick={() => void store()}
        >
          Enregistrer l’action
        </button>
        <button
          type="button"
          disabled={disabled || rec.recording || !rec.action.steps.length}
          onClick={() => void control({ mode: "clear" })}
        >
          Vider
        </button>
      </div>
      <details className="batch-settings">
        <summary>Traitement par lot</summary>
        <button
          type="button"
          disabled={disabled || batch?.running}
          onClick={() => void files()}
        >
          Choisir les fichiers ({paths.length})
        </button>
        {paths.length > 0 && (
          <p className="hint" title={paths.join("\n")}>
            {paths.slice(0, 3).map(basename).join(", ")}
            {paths.length > 3 ? "…" : ""}
          </p>
        )}
        <button
          type="button"
          disabled={disabled || batch?.running}
          onClick={() => void folder()}
        >
          Dossier de sortie…
        </button>
        <p className="hint path-label" title={directory}>
          {directory || "Aucun dossier choisi"}
        </p>
        <div className="field-grid">
          <label>
            Format du lot
            <select
              value={format}
              disabled={disabled || batch?.running}
              onChange={(e) => setFormat(e.target.value)}
            >
              {[
                "png",
                "jpg",
                "webp",
                "bmp",
                "tiff",
                "svg",
                "prisme",
                "avif",
                "gif",
                "pdf",
              ].map((v) => (
                <option key={v} value={v}>
                  {v.toUpperCase()}
                </option>
              ))}
            </select>
          </label>
          <label>
            Qualité JPEG / AVIF
            <input
              type="number"
              min={1}
              max={100}
              disabled={
                disabled || batch?.running || !["jpg", "avif"].includes(format)
              }
              value={quality}
              onChange={(e) =>
                setQuality(Math.max(1, Math.min(100, +e.target.value)))
              }
            />
          </label>
        </div>
        <p className="hint">
          Crée des copies « _prisme » dans le dossier choisi. Un suffixe évite
          les collisions. Les originaux et le document ouvert sont conservés.
        </p>
        {batch?.running ? (
          <button
            type="button"
            onClick={() =>
              void invoke("batch_cancel", { id: batch.id }).catch((e) =>
                onError(String(e)),
              )
            }
          >
            Annuler le lot
          </button>
        ) : (
          <button
            type="button"
            disabled={
              disabled ||
              rec.recording ||
              !rec.action.steps.length ||
              !paths.length ||
              !directory
            }
            onClick={() => void startBatch()}
          >
            Lancer le lot
          </button>
        )}
      </details>
      {batch && batch.id > 0 && (
        <div className="batch-progress" aria-live="polite">
          <p>
            {batch.running
              ? "Traitement"
              : batch.cancelled
                ? "Annulé"
                : "Terminé"}{" "}
            : {batch.results.length} / {batch.total} ·{" "}
            {batch.results.filter((r) => r.error).length} erreur(s)
          </p>
          <progress value={batch.results.length} max={batch.total} />
          {batch.current && (
            <p className="path-label" title={batch.current}>
              {basename(batch.current)}
            </p>
          )}
          <ul>
            {batch.results.slice(-20).map((r, i) => (
              <li key={i} title={r.output ?? r.error ?? ""}>
                {basename(r.source)} : {r.error ?? basename(r.output ?? "")}
              </li>
            ))}
          </ul>
        </div>
      )}
    </details>
  );
}
