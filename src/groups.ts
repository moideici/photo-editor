import type { Shape } from "./types";
export type Matrix = [number, number, number, number, number, number];
export type Point = [number,number];
export const identity: Matrix = [1,0,0,1,0,0];
export const multiply = (a:Matrix,b:Matrix):Matrix => [
 a[0]*b[0]+a[2]*b[1],a[1]*b[0]+a[3]*b[1],a[0]*b[2]+a[2]*b[3],a[1]*b[2]+a[3]*b[3],
 a[0]*b[4]+a[2]*b[5]+a[4],a[1]*b[4]+a[3]*b[5]+a[5]];
export const transformPoint = (m:Matrix,p:Point):Point => [m[0]*p[0]+m[2]*p[1]+m[4],m[1]*p[0]+m[3]*p[1]+m[5]];
export function inverse(m:Matrix):Matrix {
 const d=m[0]*m[3]-m[1]*m[2];
 return [m[3]/d,-m[1]/d,-m[2]/d,m[0]/d,(m[2]*m[5]-m[3]*m[4])/d,(m[1]*m[4]-m[0]*m[5])/d];
}
export function rotation(s:Shape):Matrix {
 const a=s.rotation*Math.PI/180,c=Math.cos(a),n=Math.sin(a),x=s.x+s.w/2,y=s.y+s.h/2;
 return [c,n,-n,c,x-c*x+n*y,y-n*x-c*y];
}
export const frame = (s:Shape):Matrix => multiply(multiply(s.transform??identity,rotation(s)),[s.w/s.intrinsicW!,0,0,s.h/s.intrinsicH!,s.x,s.y]);
export const matrixSvg=(m:Matrix)=>`matrix(${m.join(" ")})`;
export type Entry={shape:Shape,parent:Matrix,parentId:number|null,depth:number};
export function entries(shapes:Shape[],parent:Matrix=identity,parentId:number|null=null,depth=0):Entry[] {
 return shapes.flatMap(shape=>[{shape,parent,parentId,depth},...(shape.type==="group"?entries(shape.children??[],multiply(parent,frame(shape)),shape.id,depth+1):[])]);
}
export const context=(shapes:Shape[],id:number|null)=>entries(shapes).find(e=>e.shape.id===id);
export const findShape=(shapes:Shape[],id:number|null)=>context(shapes,id)?.shape;
export const editMatrix=(entry?:Entry):Matrix=>entry?multiply(entry.parent,entry.shape.transform??identity):identity;
export function siblings(shapes:Shape[],id:number):Shape[] {
 const entry=context(shapes,id);return entry?.parentId?findShape(shapes,entry.parentId)?.children??[]:shapes;
}
