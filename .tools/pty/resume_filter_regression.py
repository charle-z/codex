import fcntl, json, os, pty, select, shutil, signal, struct, sys, termios, time
from pathlib import Path
sys.path.insert(0, '/workspace/.tools/py')
import pyte
ROOT=Path('/workspace'); P=ROOT/'.tools/pty/final'; H=P/'codex-home'; FH=P/'fake-home'; A=P/'project-a'; B=P/'project-b'; CODEX=ROOT/'codex-rs/target/debug/codex'

def setup():
    shutil.rmtree(P, ignore_errors=True)
    sd=H/'sessions/2026/08/17'
    for d in (H,FH,A,B,sd): d.mkdir(parents=True, exist_ok=True)
    (H/'config.toml').write_text('model = \"gpt-5.6-terra\"\nmodel_provider = \"openai\"\nsuppress_unstable_features_warning = true\n\n[analytics]\nenabled = false\n\n[tui]\nresume_cwd = \"session\"\n\n'+f'[projects.\"{A}\"]\ntrust_level = \"trusted\"\n\n'+f'[projects.\"{B}\"]\ntrust_level = \"trusted\"\n')
    (H/'auth.json').write_text('{\"OPENAI_API_KEY\":\"focus-palette-test\",\"tokens\":null,\"last_refresh\":null}')
    rows=[('2026-08-17T06-10-00','2026-08-17T06:10:00Z','11111111-1111-4111-8111-111111111111',A,'PTY SESSION A'),('2026-08-17T06-11-00','2026-08-17T06:11:00Z','22222222-2222-4222-8222-222222222222',B,'PTY SESSION B')]
    for fts,ts,uid,cwd,msg in rows:
        meta={'timestamp':ts,'type':'session_meta','payload':{'session_id':uid,'id':uid,'timestamp':ts,'cwd':str(cwd),'originator':'pty-regression','cli_version':'test','source':'cli','model_provider':'openai','history_mode':'legacy'}}
        ev={'timestamp':ts,'type':'event_msg','payload':{'type':'user_message','message':msg,'kind':'plain'}}
        (sd/f'rollout-{fts}-{uid}.jsonl').write_text(json.dumps(meta)+'\n'+json.dumps(ev)+'\n')

class T:
    def __init__(self,args,cwd):
        pid,fd=pty.fork()
        if pid==0:
            env=os.environ.copy(); env.update({'TERM':'xterm-256color','OPENAI_API_KEY':'focus-palette-test','CODEX_HOME':str(H),'HOME':str(FH),'CODEX_TUI_DISABLE_KEYBOARD_ENHANCEMENT':'1'})
            os.chdir(cwd); os.execve(str(CODEX),[str(CODEX),*args],env)
        self.pid,self.fd=pid,fd; fcntl.ioctl(fd,termios.TIOCSWINSZ,struct.pack('HHHH',32,120,0,0)); self.s=pyte.Screen(120,32); self.st=pyte.Stream(self.s); self.raw=bytearray(); self.c=self.k=self.p=False
    def read(self,to=.1):
        r,_,_=select.select([self.fd],[],[],to)
        if not r:return
        try:d=os.read(self.fd,65536)
        except OSError:return
        if not d:return
        self.raw.extend(d); self.st.feed(d.decode('utf-8','ignore')); raw=bytes(self.raw)
        if not self.c and b'\x1b[6n' in raw: os.write(self.fd,b'\x1b[1;1R'); self.c=True
        if not self.k and b'\x1b[?u' in raw: os.write(self.fd,b'\x1b[?0u\x1b[?1;2c'); self.k=True
        if not self.p and b'\x1b]10;?' in raw and b'\x1b]11;?' in raw: os.write(self.fd,b'\x1b]10;rgb:ffff/ffff/ffff\x1b\\\x1b]11;rgb:0000/0000/0000\x1b\\'); self.p=True
    def text(self):return '\n'.join(self.s.display)
    def wait(self,pred,label,to=60):
        end=time.time()+to
        while time.time()<end:
            self.read(); txt=self.text()
            if pred(txt):return txt
            try:
                cp,status=os.waitpid(self.pid,os.WNOHANG)
                if cp:raise RuntimeError(f'exit {status} waiting {label}\n{txt}')
            except ChildProcessError:pass
        raise RuntimeError(f'timeout {label}\n{self.text()}')
    def send(self,b):os.write(self.fd,b)
    def close(self):
        try:os.kill(self.pid,signal.SIGTERM)
        except ProcessLookupError:pass
        try:os.waitpid(self.pid,0)
        except ChildProcessError:pass

def fl(txt):return next((x.strip() for x in txt.splitlines() if 'Filter:' in x),'')
def ma(txt):return '[All]' in fl(txt)
def mc(txt):return '[Cwd]' in fl(txt)
def picker(txt):return 'Resume a previous session' in txt or 'Filter:' in txt

def scenario(allmode):
    want='All' if allmode else 'Cwd'; t=T(['resume']+(['--all'] if allmode else [])+['--no-alt-screen'],A)
    try:
        t.wait(lambda x:picker(x) and (ma(x) if allmode else mc(x)),'startup '+want)
        if allmode:t.wait(lambda x:'PTY SESSION A' in x and 'PTY SESSION B' in x,'both startup',30)
        else:
            x=t.wait(lambda x:'PTY SESSION A' in x,'A startup',30)
            if 'PTY SESSION B' in x:raise AssertionError('B visible in startup Cwd')
        t.send(b'\r'); x=t.wait(lambda x:('OpenAI Codex' in x and not picker(x)) or 'Choose working directory to resume this session' in x,'main TUI or cwd prompt',60);
        if 'Choose working directory to resume this session' in x: t.send(b'1'); t.wait(lambda x:'OpenAI Codex' in x and not picker(x),'main TUI after cwd choice',60)
        t.send(b'/resume'); time.sleep(1.0); t.send(b'\r')
        if os.environ.get('PROBE_REOPEN') == '1':
            x=t.wait(lambda value: picker(value) and bool(fl(value)),'reopen observed',60)
            return {'scenario':want,'observed_reopen_filter':fl(x)}
        x=t.wait(lambda x:picker(x) and (ma(x) if allmode else mc(x)),'reopen '+want,60)
        if allmode:
            t.wait(lambda x:'PTY SESSION A' in x and 'PTY SESSION B' in x,'both reopened',30); t.send(b'\x1b[C'); x=t.wait(mc,'toggle Cwd',30); tog='ok'
        else:
            if 'PTY SESSION B' in x:raise AssertionError('B visible in reopened Cwd')
            tog='n/a'
        return {'scenario':want,'startup_and_reopen':'ok','toggle_all_to_cwd':tog,'final_filter':fl(x)}
    finally:
        (P/f'raw-{want.lower()}.bin').write_bytes(bytes(t.raw)); (P/f'screen-{want.lower()}.txt').write_text(t.text()); t.close()

def main():
    setup(); assert CODEX.exists();
    results=[scenario(True)] if os.environ.get('PROBE_REOPEN') == '1' else [scenario(True),scenario(False)]
    print(json.dumps(results,indent=2))
if __name__=='__main__':main()
