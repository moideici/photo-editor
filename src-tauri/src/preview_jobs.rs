//! Cooperative cancellation belongs to the isolated preview, never the live editor.
use std::{cell::RefCell, collections::HashMap, sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}}};
#[derive(Clone, Default)]
pub struct Token(Arc<AtomicBool>);
impl Token {
    pub fn cancel(&self) { self.0.store(true, Ordering::Relaxed); }
    pub fn cancelled(&self) -> bool { self.0.load(Ordering::Relaxed) }
}
thread_local! { static CURRENT: RefCell<Option<Token>> = const { RefCell::new(None) }; }
pub fn current() -> Token { CURRENT.with(|c| c.borrow().clone().unwrap_or_default()) }
pub fn checkpoint() -> Result<(), String> {
    if CURRENT.with(|c|c.borrow().as_ref().is_some_and(Token::cancelled)) {Err("Aperçu annulé".into())} else {Ok(())}
}
pub fn run<T>(token: Token, work: impl FnOnce()->Result<T,String>) -> Result<T,String> {
    struct Restore(Option<Token>,Token);
    impl Drop for Restore {fn drop(&mut self){self.1.cancel();CURRENT.with(|c|*c.borrow_mut()=self.0.take());}}
    let old=CURRENT.with(|c|c.replace(Some(token.clone())));
    let _restore=Restore(old,token);
    checkpoint()?;
    let result=work()?;
    checkpoint()?;
    Ok(result)
}
#[derive(Default)]
pub struct Jobs {
    entries: Mutex<HashMap<String,(u64,Token)>>,
    // A single full-resolution preview bounds transient pixel allocations.
    pub worker: Mutex<()>,
}
impl Jobs {
    pub fn begin(&self, owner:&str, request:u64)->Result<Token,String>{
        if owner.is_empty()||owner.len()>128{return Err("Identifiant d’aperçu invalide".into());}
        let mut jobs=self.entries.lock().map_err(|_|"Aperçus indisponibles")?;
        if let Some((previous,token))=jobs.get(owner){
            if request<=*previous{return Err("Aperçu remplacé".into());}
            token.cancel();
        } else if jobs.len()>=128 {
            // Completed and cancelled owners may be forgotten; active owners stay bounded.
            jobs.retain(|_,(_,token)|!token.cancelled());
            if jobs.len()>=128{return Err("Trop d’aperçus simultanés".into());}
        }
        let token=Token::default();jobs.insert(owner.into(),(request,token.clone()));Ok(token)
    }
    pub fn cancel(&self, owner:&str, request:u64)->Result<(),String>{
        if owner.is_empty()||owner.len()>128{return Err("Identifiant d’aperçu invalide".into());}
        let mut jobs=self.entries.lock().map_err(|_|"Aperçus indisponibles")?;
        if let Some((latest,token))=jobs.get(owner){if request>=*latest {token.cancel();} if request<=*latest{return Ok(());}}
        if jobs.len()<128{let token=Token::default();token.cancel();jobs.insert(owner.into(),(request,token));}
        Ok(())
    }
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn replacement_and_late_cancellation_preserve_the_latest_request(){
        let jobs=Jobs::default();let first=jobs.begin("dialog",1).unwrap();let second=jobs.begin("dialog",2).unwrap();
        assert!(first.cancelled());assert!(!second.cancelled());jobs.cancel("dialog",1).unwrap();assert!(!second.cancelled());
        jobs.cancel("dialog",2).unwrap();assert!(second.cancelled());assert!(jobs.begin("dialog",1).is_err());
        jobs.cancel("late",4).unwrap();assert!(jobs.begin("late",4).is_err());assert!(jobs.begin("late",5).is_ok());
    }
    #[test] fn cancelled_parallel_kernel_stops_work_and_scope_restores(){
        let token=Token::default();let cancel=token.clone();let counter=Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counted=counter.clone();let img=image::RgbaImage::from_pixel(2048,1024,image::Rgba([17,43,85,255]));
        let result=run(token,||{
            let out=crate::raster::map_preview_test(&img,|p|{if counted.fetch_add(1,Ordering::Relaxed)>1000{cancel.cancel();}p});
            Ok(out)
        });
        assert!(result.is_err());assert!(counter.load(Ordering::Relaxed)<100_000);assert!(checkpoint().is_ok());
        // Normal processing remains exact after cancellation; live operations have no token.
        assert_eq!(crate::raster::filter(&img,"invert",&serde_json::json!({})).unwrap().get_pixel(100,100).0,[238,212,170,255]);
    }
}
