import { useEffect, useRef, useState } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import type {
  LayerEffects,
  ShadowEffect,
  OutlineEffect,
  BevelEffect,
} from "./types";
export const emptyEffects = (): LayerEffects => ({
  shadow: null,
  outline: null,
  bevel: null,
});
const shadowDefault = (): ShadowEffect => ({
  x: 8,
  y: 8,
  blur: 4,
  spread: 0,
  color: "#000000",
  opacity: 0.6,
});
const outlineDefault = (): OutlineEffect => ({
  width: 3,
  color: "#000000",
  opacity: 1,
  position: "outside",
});
const bevelDefault = (): BevelEffect => ({
  size: 4,
  depth: 4,
  angle: 225,
  opacity: 0.75,
  highlight: "#ffffff",
  shade: "#000000",
});
function NumberControl({
  label,
  value,
  onChange,
  min,
  max,
  step = 1,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
  min: number;
  max: number;
  step?: number;
}) {
  return (
    <label>
      {label}
      <input
        aria-label={label}
        type="number"
        required
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(+e.target.value)}
      />
    </label>
  );
}
function Opacity({
  label,
  value,
  onChange,
}: {
  label: string;
  value: number;
  onChange: (value: number) => void;
}) {
  return (
    <label>
      {label} : {Math.round(value * 100)} %
      <input
        aria-label={label}
        type="range"
        min={0}
        max={100}
        value={Math.round(value * 100)}
        onChange={(e) => onChange(+e.target.value / 100)}
      />
    </label>
  );
}
export default function EffectsEditor({
  value,
  onChange,
  layerId,
}: {
  value: LayerEffects;
  onChange: (effects: LayerEffects) => void;
  layerId: number;
}) {
  const [preview, setPreview] = useState(""),
    [error, setError] = useState(""),
    [pending, setPending] = useState(false);
  const presets = useRef({
    shadow: value.shadow ?? shadowDefault(),
    outline: value.outline ?? outlineDefault(),
    bevel: value.bevel ?? bevelDefault(),
  });
  useEffect(() => {
    let cancelled = false;
    if (!isTauri()) return;
    setPending(true);
    const timer = setTimeout(() => {
      void invoke<string>("preview_effects", { id: layerId, effects: value })
        .then((src) => {
          if (!cancelled) {
            setPreview(src);
            setError("");
          }
        })
        .catch((e) => {
          if (!cancelled) setError(String(e));
        })
        .finally(() => {
          if (!cancelled) setPending(false);
        });
    }, 150);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [value, layerId]);
  const shadow = value.shadow,
    outline = value.outline,
    bevel = value.bevel;
  const patchShadow = (p: Partial<ShadowEffect>) =>
    onChange({ ...value, shadow: { ...shadow!, ...p } });
  const patchOutline = (p: Partial<OutlineEffect>) =>
    onChange({ ...value, outline: { ...outline!, ...p } });
  const patchBevel = (p: Partial<BevelEffect>) =>
    onChange({ ...value, bevel: { ...bevel!, ...p } });
  return (
    <div className="effects-editor">
      <div className="effect-preview checker" aria-busy={pending}>
        {preview && <img src={preview} alt="Aperçu des effets" />}
      </div>
      <fieldset>
        <legend>
          <label className="check-label">
            <input
              type="checkbox"
              checked={!!shadow}
              onChange={(e) => {
                if (shadow) presets.current.shadow = shadow;
                onChange({
                  ...value,
                  shadow: e.target.checked ? presets.current.shadow : null,
                });
              }}
            />
            Ombre portée
          </label>
        </legend>
        {shadow && (
          <>
            <div className="field-grid">
              <NumberControl
                label="Décalage horizontal"
                value={shadow.x}
                min={-512}
                max={512}
                onChange={(x) => patchShadow({ x })}
              />
              <NumberControl
                label="Décalage vertical"
                value={shadow.y}
                min={-512}
                max={512}
                onChange={(y) => patchShadow({ y })}
              />
              <NumberControl
                label="Flou de l’ombre"
                value={shadow.blur}
                min={0}
                max={128}
                step={0.5}
                onChange={(blur) => patchShadow({ blur })}
              />
              <NumberControl
                label="Étalement"
                value={shadow.spread}
                min={0}
                max={128}
                step={0.5}
                onChange={(spread) => patchShadow({ spread })}
              />
            </div>
            <label>
              Couleur de l’ombre
              <input
                aria-label="Couleur de l’ombre"
                type="color"
                value={shadow.color.slice(0, 7)}
                onChange={(e) => patchShadow({ color: e.target.value })}
              />
            </label>
            <Opacity
              label="Opacité de l’ombre"
              value={shadow.opacity}
              onChange={(opacity) => patchShadow({ opacity })}
            />
          </>
        )}
      </fieldset>
      <fieldset>
        <legend>
          <label className="check-label">
            <input
              type="checkbox"
              checked={!!outline}
              onChange={(e) => {
                if (outline) presets.current.outline = outline;
                onChange({
                  ...value,
                  outline: e.target.checked ? presets.current.outline : null,
                });
              }}
            />
            Contour du calque
          </label>
        </legend>
        {outline && (
          <>
            <div className="field-grid">
              <NumberControl
                label="Largeur du contour"
                value={outline.width}
                min={0.1}
                max={128}
                step={0.1}
                onChange={(width) => patchOutline({ width })}
              />
              <label>
                Position
                <select
                  aria-label="Position du contour"
                  value={outline.position}
                  onChange={(e) =>
                    patchOutline({
                      position: e.target.value as OutlineEffect["position"],
                    })
                  }
                >
                  <option value="outside">Extérieur</option>
                  <option value="inside">Intérieur</option>
                  <option value="center">Centré</option>
                </select>
              </label>
            </div>
            <label>
              Couleur du contour
              <input
                aria-label="Couleur du contour"
                type="color"
                value={outline.color.slice(0, 7)}
                onChange={(e) => patchOutline({ color: e.target.value })}
              />
            </label>
            <Opacity
              label="Opacité du contour"
              value={outline.opacity}
              onChange={(opacity) => patchOutline({ opacity })}
            />
          </>
        )}
      </fieldset>
      <fieldset>
        <legend>
          <label className="check-label">
            <input
              type="checkbox"
              checked={!!bevel}
              onChange={(e) => {
                if (bevel) presets.current.bevel = bevel;
                onChange({
                  ...value,
                  bevel: e.target.checked ? presets.current.bevel : null,
                });
              }}
            />
            Biseau
          </label>
        </legend>
        {bevel && (
          <>
            <div className="field-grid">
              <NumberControl
                label="Taille du biseau"
                value={bevel.size}
                min={0.5}
                max={128}
                step={0.5}
                onChange={(size) => patchBevel({ size })}
              />
              <NumberControl
                label="Profondeur"
                value={bevel.depth}
                min={0}
                max={32}
                step={0.5}
                onChange={(depth) => patchBevel({ depth })}
              />
              <NumberControl
                label="Angle de lumière"
                value={bevel.angle}
                min={-360}
                max={360}
                onChange={(angle) => patchBevel({ angle })}
              />
              <label>
                Reflets
                <input
                  aria-label="Couleur des reflets"
                  type="color"
                  value={bevel.highlight.slice(0, 7)}
                  onChange={(e) => patchBevel({ highlight: e.target.value })}
                />
              </label>
              <label>
                Ombres
                <input
                  aria-label="Couleur des ombres du biseau"
                  type="color"
                  value={bevel.shade.slice(0, 7)}
                  onChange={(e) => patchBevel({ shade: e.target.value })}
                />
              </label>
            </div>
            <Opacity
              label="Intensité du biseau"
              value={bevel.opacity}
              onChange={(opacity) => patchBevel({ opacity })}
            />
          </>
        )}
      </fieldset>
      <button type="button" onClick={() => onChange(emptyEffects())}>
        Retirer tous les effets
      </button>
      <p className="hint">
        Les effets restent modifiables. Ils suivent le contenu du calque, y
        compris le texte vectoriel, et sont conservés dans le projet et le SVG.
      </p>
      {error && <p role="alert">{error}</p>}
    </div>
  );
}
