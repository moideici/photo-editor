import type { Shape, VectorClip } from "./types";
import { inverse, transformPoint } from "./groups";
export type Point = [number, number];
let hitContext: CanvasRenderingContext2D | null | undefined;
export function rotatePoint(
  point: Point,
  center: Point,
  degrees: number,
): Point {
  const a = (degrees * Math.PI) / 180,
    c = Math.cos(a),
    s = Math.sin(a),
    x = point[0] - center[0],
    y = point[1] - center[1];
  return [center[0] + c * x - s * y, center[1] + s * x + c * y];
}
export function shapeHit(
  shape: Shape,
  point: Point,
  tolerance: number,
  clipGeometry = false,
): boolean {
  if(shape.opacity===0)return false;
  // Native font metrics, including variations, can extend past the nominal
  // text frame. Let the native picker test the actual visible glyph pixel.
  if(shape.type==='text')return true;
  const p = rotatePoint(
    shape.transform?transformPoint(inverse(shape.transform),point):point,
    [shape.x + shape.w / 2, shape.y + shape.h / 2],
    -shape.rotation,
  );
  if (shape.type === "group") {
    const sx=shape.w/shape.intrinsicW!,sy=shape.h/shape.intrinsicH!;
    const local:Point=[(p[0]-shape.x)/sx,(p[1]-shape.y)/sy];
    if(shape.clip&&!clipContains(shape.clip,local))return false;
    if(shape.filters&&!clipGeometry){
      const q=transformPoint(inverse(shape.filters.transform),local),last=shape.filters.filters.at(-1);
      if(last){const [x,y,w,h]=last.region;return q[0]>=x&&q[1]>=y&&q[0]<=x+w&&q[1]<=y+h;}
    }
    return (shape.children??[]).some(s=>shapeHit(s,local,tolerance/Math.max(.0001,Math.min(sx,sy)),clipGeometry));
  }
  if (shape.type === "line") {
    const dx = shape.w,
      dy = shape.h,
      len = dx * dx + dy * dy;
    const t = len
      ? Math.max(
          0,
          Math.min(1, ((p[0] - shape.x) * dx + (p[1] - shape.y) * dy) / len),
        )
      : 0;
    return (
      Math.hypot(p[0] - shape.x - t * dx, p[1] - shape.y - t * dy) <=
      Math.max(tolerance, shape.strokeWidth / 2)
    );
  }
  if (shape.type === "path") {
    hitContext ??= document.createElement("canvas").getContext("2d");
    if (hitContext) {
      const path = new Path2D();
      const xy = (v: Point): Point => [
        shape.x + v[0] * shape.w,
        shape.y + v[1] * shape.h,
      ];
      for (const c of shape.contours ?? []) {
        if (!c.nodes.length) continue;
        path.moveTo(...xy(c.nodes[0].point));
        for (let i = 1; i < c.nodes.length + Number(c.closed); i++) {
          const a = c.nodes[i - 1],
            b = c.nodes[i % c.nodes.length];
          path.bezierCurveTo(
            ...xy(a.outgoing),
            ...xy(b.incoming),
            ...xy(b.point),
          );
        }
        if (c.closed) path.closePath();
      }
      hitContext.lineWidth =
        (shape.stroke || shape.strokeGradient || shape.strokePattern ? shape.strokeWidth : 0) + 2 * tolerance;
      hitContext.lineCap = "round";
      hitContext.lineJoin = "miter";
      return (
        ((clipGeometry || !!shape.fill || !!shape.fillGradient || !!shape.fillPattern) &&
          hitContext.isPointInPath(
            path,
            ...p,
            shape.evenOdd ? "evenodd" : "nonzero",
          )) ||
        (!clipGeometry && (!!shape.stroke || !!shape.fill || !!shape.fillGradient || !!shape.fillPattern || !!shape.strokeGradient || !!shape.strokePattern) &&
          hitContext.isPointInStroke(path, ...p))
      );
    }
  }
  return (
    p[0] >= shape.x - tolerance &&
    p[1] >= shape.y - tolerance &&
    p[0] <= shape.x + shape.w + tolerance &&
    p[1] <= shape.y + shape.h + tolerance
  );
}
export function clipContains(clip:VectorClip,point:Point):boolean {
  const local=transformPoint(inverse(clip.transform),point);
  return clip.paths.some(s=>shapeHit(s,local,0,true))&&(!clip.intersect||clipContains(clip.intersect,point));
}
export function selectedShapeHit(shapes:Shape[],id:number,point:Point,tolerance:number):boolean {
  for(const s of shapes){
    if(s.id===id)return shapeHit(s,point,tolerance);
    if(s.type==='group'&&s.opacity!==0){
      const p=rotatePoint(s.transform?transformPoint(inverse(s.transform),point):point,[s.x+s.w/2,s.y+s.h/2],-s.rotation);
      const local:Point=[(p[0]-s.x)*s.intrinsicW!/s.w,(p[1]-s.y)*s.intrinsicH!/s.h];
      if((!s.clip||clipContains(s.clip,local))&&selectedShapeHit(s.children??[],id,local,tolerance*Math.max(s.intrinsicW!/s.w,s.intrinsicH!/s.h)))return true;
    }
  }
  return false;
}
export function hitShape(shapes:Shape[],point:Point,tolerance:number,deep=false):Shape|undefined {
 for(const s of [...shapes].reverse())if(shapeHit(s,point,tolerance)){
  if(deep&&s.type==="group"){
   const p=rotatePoint(s.transform?transformPoint(inverse(s.transform),point):point,[s.x+s.w/2,s.y+s.h/2],-s.rotation);
   return hitShape(s.children??[],[(p[0]-s.x)*s.intrinsicW!/s.w,(p[1]-s.y)*s.intrinsicH!/s.h],tolerance*Math.max(s.intrinsicW!/s.w,s.intrinsicH!/s.h),true)??s;
  }
  return s;
 }
}
export function hitCandidates(shapes:Shape[],point:Point,tolerance:number,deep=false):Shape[]{
  const out:Shape[]=[];
  for(const s of [...shapes].reverse())if(shapeHit(s,point,tolerance)){
    if(deep&&s.type==='group'){
      const p=rotatePoint(s.transform?transformPoint(inverse(s.transform),point):point,[s.x+s.w/2,s.y+s.h/2],-s.rotation);
      out.push(...hitCandidates(s.children??[],[(p[0]-s.x)*s.intrinsicW!/s.w,(p[1]-s.y)*s.intrinsicH!/s.h],tolerance*Math.max(s.intrinsicW!/s.w,s.intrinsicH!/s.h),true));
    }
    out.push(s);
  }
  return out;
}
export function resizeShape(shape: Shape, delta: Point): Shape {
  const [dx, dy] = rotatePoint(delta, [0, 0], -shape.rotation);
  if (shape.type === "line")
    return { ...shape, w: shape.w + dx, h: shape.h + dy };
  const w = Math.max(1, shape.w + dx),
    h = Math.max(1, shape.h + dy);
  const anchor = rotatePoint(
    [shape.x, shape.y],
    [shape.x + shape.w / 2, shape.y + shape.h / 2],
    shape.rotation,
  );
  const [ox, oy] = rotatePoint([w / 2, h / 2], [0, 0], shape.rotation);
  return {
    ...shape,
    x: anchor[0] + ox - w / 2,
    y: anchor[1] + oy - h / 2,
    w,
    h,
  };
}
