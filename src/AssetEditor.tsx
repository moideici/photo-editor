import { useEffect, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
export type AssetKind = "mask" | "frame" | "pattern" | "texture";
export interface AssetOptions {
  kind: AssetKind;
  name: string;
  scale: number;
  strength: number;
  color: string;
  secondary: string;
  inverted: boolean;
  path: string | null;
}
const catalog: Record<AssetKind, [string, string][]> = {
  mask: [
    ["ellipse", "Ellipse"],
    ["rounded", "Coins arrondis"],
    ["radial", "Fondu radial"],
    ["horizontal", "Fondu horizontal"],
    ["vertical", "Fondu vertical"],
    ["diagonal", "Fondu diagonal"],
  ],
  frame: [
    ["plain", "Simple"],
    ["double", "Double"],
    ["film", "Pellicule"],
    ["bevel", "Biseau"],
  ],
  pattern: [
    ["checker", "Damier"],
    ["stripes", "Rayures"],
    ["dots", "Points"],
    ["bricks", "Briques"],
  ],
  texture: [
    ["paper", "Papier"],
    ["canvas", "Toile"],
    ["grain", "Grain"],
    ["wood", "Bois"],
  ],
};
export function assetDefaults(
  kind: AssetKind,
  color: string,
  secondary: string,
): AssetOptions {
  return {
    kind,
    name: catalog[kind][0][0],
    scale: kind === "frame" ? 10 : 16,
    strength: kind === "texture" ? 0.5 : 1,
    color,
    secondary,
    inverted: false,
    path: null,
  };
}
export default function AssetEditor({
  value,
  onChange,
}: {
  value: AssetOptions;
  onChange: (v: AssetOptions) => void;
}) {
  const [preview, setPreview] = useState(""),
    [error, setError] = useState("");
  const patch = (p: Partial<AssetOptions>) => onChange({ ...value, ...p });
  useEffect(() => {
    let cancelled = false;
    if (!isTauri()) return;
    const timer = setTimeout(() => {
      if (!Number.isFinite(value.scale) || value.scale < 1 || value.scale > 512)
        return;
      void invoke<string>("asset_preview", { settings: value })
        .then((src) => {
          if (!cancelled) {
            setPreview(src);
            setError("");
          }
        })
        .catch((e) => {
          if (!cancelled) setError(String(e));
        });
    }, 120);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [value]);
  return (
    <div className="asset-editor">
      <label>
        Ressource
        <select
          aria-label="Ressource"
          value={value.path ? "custom" : value.name}
          onChange={(e) => patch({ name: e.target.value, path: null })}
        >
          {catalog[value.kind].map(([name, label]) => (
            <option value={name} key={name}>
              {label}
            </option>
          ))}
          {value.path && <option value="custom">Image chargée</option>}
        </select>
      </label>
      <div className="asset-preview checker">
        {preview && (
          <img
            src={preview}
            alt="Aperçu de la ressource"
            width={180}
            height={120}
          />
        )}
      </div>
      {(value.kind !== "mask" || value.name === "rounded") && (
        <label>
          {value.kind === "frame" ? "Largeur (px)" : "Échelle (px)"}
          <input
            aria-label="Échelle de la ressource"
            required
            type="number"
            min={1}
            max={512}
            value={value.scale}
            onChange={(e) => patch({ scale: +e.target.value })}
          />
        </label>
      )}
      <label>
        Intensité : {Math.round(value.strength * 100)} %
        <input
          aria-label="Intensité de la ressource"
          type="range"
          min={0}
          max={100}
          value={Math.round(value.strength * 100)}
          onChange={(e) => patch({ strength: +e.target.value / 100 })}
        />
      </label>
      {value.kind === "mask" && (
        <label className="check-label">
          <input
            type="checkbox"
            checked={value.inverted}
            onChange={(e) => patch({ inverted: e.target.checked })}
          />
          Inverser le masque
        </label>
      )}
      {["frame", "pattern"].includes(value.kind) && (
        <div className="field-grid">
          <label>
            Couleur
            <input
              type="color"
              value={value.color}
              onChange={(e) => patch({ color: e.target.value })}
            />
          </label>
          <label>
            Seconde couleur
            <input
              type="color"
              value={value.secondary}
              onChange={(e) => patch({ secondary: e.target.value })}
            />
          </label>
        </div>
      )}
      <button
        type="button"
        onClick={() =>
          void (async () => {
            try {
              const path = await open({
                title: "Charger une ressource",
                multiple: false,
                filters: [
                  {
                    name: "Images",
                    extensions: [
                      "png",
                      "jpg",
                      "jpeg",
                      "webp",
                      "bmp",
                      "tiff",
                      "svg",
                    ],
                  },
                ],
              });
              if (typeof path === "string") patch({ path, name: "custom" });
            } catch (e) {
              setError(String(e));
            }
          })()
        }
      >
        Charger une image…
      </button>
      {value.path && (
        <span className="hint">{value.path.split(/[\\/]/).at(-1)}</span>
      )}
      <p className="hint">
        La bibliothèque intégrée est créée pour Prisme. La ressource est
        appliquée au calque bitmap actif, dans la sélection courante.
      </p>
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
