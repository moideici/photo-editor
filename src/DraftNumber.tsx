import {useEffect,useState} from 'react';
export default function DraftNumber({label,value,onChange}:{label:string;value:number;onChange:(value:number)=>void}){
 const [text,setText]=useState(String(value));useEffect(()=>setText(String(value)),[value]);
 return <label>{label}<input aria-label={label} type="text" inputMode="decimal" value={text} onChange={e=>{const text=e.target.value;setText(text);const n=Number(text.replace(',','.'));if(text.trim()&&Number.isFinite(n))onChange(n);}} onBlur={()=>setText(String(value))}/></label>;
}
