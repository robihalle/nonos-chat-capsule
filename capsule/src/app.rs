// SPDX-License-Identifier: AGPL-3.0-or-later
use alloc::{string::{String,ToString},vec::Vec,format};
use nonos_app_skeleton::{App,AppManifest,WindowKind,InputEvent,InputKind,EventOutcome,PaintBuffer,KEY_TAB,KEY_ENTER,KEY_BACKSPACE,KEY_ESC,KEY_UP,KEY_DOWN,MOD_CTRL,MOD_SHIFT,clipboard_paste};
use zeroize::Zeroize;
use crate::{protocol::{self,Endpoint,Message},transport::Job};
pub struct Chat {
    fields:[String;4], focus:usize, selected:bool, messages:Vec<Message>,
    status:String, result:String, job:Option<Job>, models:bool, scroll:usize,
    width:u32,height:u32,
}
impl Chat {
    pub fn new()->Self {
        Self {fields:["https://api.openai.com/v1".into(),String::with_capacity(4096),String::new(),String::new()],
            focus:0,selected:false,messages:Vec::new(),status:"API-Adresse, Key und Modell eintragen.".into(),
            result:String::new(),job:None,models:false,scroll:0,width:1040,height:660}
    }
    fn start(&mut self,models:bool) {
        if self.job.is_some(){self.job=None;self.status="Anfrage abgebrochen.".into();return;}
        let ep=match Endpoint::parse(&self.fields[0]){Ok(e)=>e,Err(e)=>{self.status=e.into();return;}};
        if !models && self.fields[3].trim().is_empty(){self.status="Bitte eine Nachricht eingeben.".into();return;}
        let mut rows=self.messages.clone();
        if !models {rows.push(Message{role:"user",text:self.fields[3].clone()});}
        match protocol::request(&ep,&self.fields[1],&self.fields[2],&rows,models) {
            Ok(r)=>{self.job=Some(Job::new(ep,r));self.models=models;self.status=if models{"Verbindung wird geprueft ..."}else{"Modell antwortet ... (Abbrechen jederzeit moeglich)"}.into();}
            Err(e)=>self.status=e.into(),
        }
    }
    fn edit(&mut self,text:&str) {
        if self.job.is_some(){return;}
        if self.selected {self.fields[self.focus].zeroize();self.selected=false;}
        let limit=if self.focus==3{8000}else if self.focus==2{256}else{4096};
        for c in text.chars().filter(|c|!c.is_control() || (*c=='\n' && self.focus==3)) {
            if self.fields[self.focus].len()+c.len_utf8()>limit{break;} self.fields[self.focus].push(c);
        }
        if self.focus<3 {self.messages.clear();self.result.clear();self.status="Einstellungen geaendert. Neuer Chat.".into();}
    }
    fn key(&mut self,e:InputEvent) {
        if e.flags & MOD_CTRL != 0 {
            match e.code {
                97|65=>self.selected=true,
                118|86=>{let mut b=[0u8;8192];if let Ok(n)=clipboard_paste(&mut b){if let Ok(s)=core::str::from_utf8(&b[..n]){self.edit(s);}} b.zeroize();}
                _=>{}
            }
            return;
        }
        match e.code {
            KEY_TAB=>{self.focus=(self.focus+1)%4;self.selected=false;}
            KEY_ENTER if self.focus==3 && e.flags&MOD_SHIFT==0=>self.start(false),
            KEY_ENTER if self.focus==3=>self.edit("\n"),
            KEY_ENTER=>{self.focus=(self.focus+1)%4;self.selected=false;}
            KEY_ESC=>{self.job=None;self.status="Anfrage abgebrochen.".into();}
            KEY_BACKSPACE=>{if self.job.is_none(){if self.selected{self.fields[self.focus].zeroize();self.selected=false;}else{self.fields[self.focus].pop();}if self.focus<3{self.messages.clear();self.result.clear();}}}
            KEY_UP=>self.scroll=self.scroll.saturating_sub(2),
            KEY_DOWN=>self.scroll=self.scroll.saturating_add(2),
            c=>if let Some(b)=crate::keymap::printable(c,e.flags&MOD_SHIFT!=0){self.edit(core::str::from_utf8(&[b]).unwrap_or(""));},
        }
    }
}
impl Drop for Chat {fn drop(&mut self){self.job=None;self.fields[1].zeroize();}}
impl App for Chat {
    fn manifest(&self)->AppManifest {AppManifest{title:b"NONOS Chat",window_id:0x43484154,kind:WindowKind::Normal,initial_x:32,initial_y:48,width:1040,height:660,input_kind_mask:1|8|16|32}}
    fn on_event(&mut self,e:InputEvent)->EventOutcome {
        match e.kind {
            InputKind::KeyDown=>self.key(e),
            InputKind::ButtonDown=>{
                let x=e.x;let y=e.y;
                if (82..126).contains(&y){self.focus=0;self.selected=false;}
                else if (156..200).contains(&y){self.focus=if x<(self.width as i32*3/5){1}else{2};self.selected=false;}
                else if (218..258).contains(&y){
                    if x<220{self.start(true);}
                    else if x<420{self.job=None;self.messages.clear();self.result.clear();self.fields[3].clear();self.scroll=0;self.status="Neuer Chat.".into();}
                    else if x<640{self.job=None;self.fields[1].zeroize();self.status="API-Key geloescht.".into();}
                } else if y>=self.height as i32-100 && y<self.height as i32-40 {
                    if x>=self.width as i32-150{self.start(false);}else{self.focus=3;self.selected=false;}
                }
            }
            InputKind::Wheel=>{if e.delta_y>0{self.scroll=self.scroll.saturating_add(3);}else{self.scroll=self.scroll.saturating_sub(3);}}
            _=>return EventOutcome::Idle,
        }
        EventOutcome::Repaint
    }
    fn on_tick(&mut self)->bool {
        let Some(job)=self.job.as_mut() else{return false;};
        match job.poll() {
            Ok(None)=>false,
            Ok(Some(r))=>{
                self.job=None;
                match protocol::answer(r,self.models){
                    Ok(s) if self.models=>{self.result=s;self.status="Verbindung erfolgreich. Modellnamen unten ablesen und oben eintragen.".into();self.scroll=0;}
                    Ok(s)=>{
                        self.messages.push(Message{role:"user",text:core::mem::take(&mut self.fields[3])});
                        self.messages.push(Message{role:"assistant",text:s});self.result.clear();
                        self.status="Antwort empfangen.".into();self.focus=3;self.scroll=usize::MAX;
                    }
                    Err(e)=>self.status=e,
                }
                true
            }
            Err(e)=>{self.job=None;self.status=e.into();true}
        }
    }
    fn tick_interval_ms(&self)->i64{25}
    fn busy(&self)->bool{self.job.is_some()}
    fn paint(&mut self,fb:&mut PaintBuffer) {
        self.width=fb.width;self.height=fb.height;
        let w=fb.width;let h=fb.height;
        fb.fill_rect(0,0,w,h,0xff0d1320);
        fb.text_ttf(24,16,"NONOS",0xff63e2c6,23.0);
        fb.text_ttf(123,16,"Chat",0xffeef5ff,23.0);
        fb.text_ttf(w.saturating_sub(240) as i32,24,"Privater Sitzungschat",0xff9babbd,14.0);
        fb.text_ttf(24,58,"API-Adresse",0xffa8b5c6,14.0);
        let kw=w*3/5;
        fb.text_ttf(24,132,"API-Key",0xffa8b5c6,14.0);
        fb.text_ttf(kw as i32+12,132,"Modell",0xffa8b5c6,14.0);
        field(fb,24,82,w.saturating_sub(48),44,&self.fields[0],self.focus==0,self.selected&&self.focus==0);
        let mask="*".repeat(self.fields[1].chars().count().min(50));
        field(fb,24,156,kw.saturating_sub(36),44,&mask,self.focus==1,self.selected&&self.focus==1);
        field(fb,kw+12,156,w.saturating_sub(kw+36),44,&self.fields[2],self.focus==2,self.selected&&self.focus==2);
        button(fb,24,218,180,"Verbindung testen",false);
        button(fb,220,218,175,"Neuer Chat",false);
        button(fb,410,218,200,"Key entfernen",false);
        let top=280;let bottom=h.saturating_sub(120);
        fb.fill_rect(24,top,w.saturating_sub(48),bottom.saturating_sub(top),0xff131e2d);
        let mut lines:Vec<(bool,String)>=Vec::new();
        let chars=(w.saturating_sub(100)/9).max(10) as usize;
        if !self.result.is_empty(){wrap(&self.result,chars,false,&mut lines);}
        else if self.messages.is_empty(){
            wrap("Dein KI-Chat in NONOS. Trage deine API-Daten ein und sende eine Nachricht.",chars,false,&mut lines);
            wrap("Key und Verlauf bleiben nur fuer diese Sitzung im RAM.",chars,false,&mut lines);
        } else {
            for m in &self.messages {
                lines.push((true,if m.role=="user"{"Du".into()}else{"Assistent".into()}));
                wrap(&m.text,chars,false,&mut lines);lines.push((false,String::new()));
            }
        }
        let visible=bottom.saturating_sub(top+20) as usize/23;
        self.scroll=self.scroll.min(lines.len().saturating_sub(visible.max(1)));
        for (i,(label,line)) in lines.iter().skip(self.scroll).take(visible).enumerate(){
            fb.text_ttf(40,(top+10) as i32+(i*23) as i32,line,if *label{0xff63e2c6}else{0xffdde6f2},16.0);
        }
        let y=h.saturating_sub(100);
        field(fb,24,y,w.saturating_sub(190),54,&self.fields[3].replace('\n'," / "),self.focus==3,self.selected&&self.focus==3);
        button(fb,w.saturating_sub(150),y+6,126,if self.job.is_some(){"Abbrechen"}else{"Senden"},true);
        let status: String=self.status.chars().take((w/7) as usize).collect();
        fb.text_ttf(24,h.saturating_sub(32) as i32,&status,0xffa8b5c6,13.0);
    }
}
fn button(fb:&mut PaintBuffer,x:u32,y:u32,w:u32,label:&str,accent:bool){
    fb.fill_rect(x,y,w,40,if accent{0xff63e2c6}else{0xff263348});
    fb.text_ttf(x as i32+12,y as i32+10,label,if accent{0xff0d1320}else{0xffeef5ff},15.0);
}
fn field(fb:&mut PaintBuffer,x:u32,y:u32,w:u32,h:u32,value:&str,focus:bool,selected:bool){
    fb.fill_rect(x,y,w,h,if focus{0xff63e2c6}else{0xff334155});
    fb.fill_rect(x+1,y+1,w.saturating_sub(2),h.saturating_sub(2),if selected{0xff294667}else{0xff182334});
    let max=(w.saturating_sub(28)/9) as usize;
    let skip=value.chars().count().saturating_sub(max);
    let tail:String=value.chars().skip(skip).take(max).collect();
    fb.text_ttf(x as i32+12,y as i32+12,&tail,0xffeef5ff,16.0);
}
fn wrap(text:&str,width:usize,label:bool,out:&mut Vec<(bool,String)>){
    for paragraph in text.split('\n'){
        let mut line=String::new();let mut count=0;
        for c in paragraph.chars(){if count>=width{out.push((label,core::mem::take(&mut line)));count=0;}line.push(c);count+=1;}
        out.push((label,line));
    }
}
