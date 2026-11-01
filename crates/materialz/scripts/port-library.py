"""Rebuild curated tile ports from shaders/original; then run generate-library.py.

Preserves the upstream formulas while lowering GLSL syntax to typed tile programs.
Whole-material mixers and retired experimental files are inventoried separately.
"""
import re,json
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
TYPES={'float':1,'int':1,'bool':1,'vec2':2,'vec3':3,'vec4':4,'mat2':4}
TOK=re.compile(r'0x[0-9a-fA-F]+[uU]?|(?:\d*\.\d+|\d+\.?\d*)(?:[eE][+-]?\d+)?[fFuU]?|[A-Za-z_]\w*|>>|<<|\+\+|--|\+=|-=|\*=|/=|==|!=|<=|>=|&&|\|\||[^\s]')
class V:
 def __init__(self,s,w=1,mat=False):
  self.s,self.w,self.mat=s,w,mat;self.origin=s
  try:self.constant=float(s)
  except:self.constant=None
 def __str__(self):return self.s
 def lane(self,i):return self.s if self.w==1 else f'({self.s}).{"xyzw"[i]}'
def vec(args,n=None):
 n=n or sum(a.w for a in args)
 return V(f'vec{n}('+', '.join(str(a) for a in args)+')',n)
def binary(op,a,b):
 if op=='*' and (a.mat or b.mat):
  m,v=(a,b) if a.mat else (b,a)
  if a.mat:return vec([V(f'{m.lane(0)}*{v.lane(0)}+{m.lane(2)}*{v.lane(1)}'),V(f'{m.lane(1)}*{v.lane(0)}+{m.lane(3)}*{v.lane(1)}')])
  return vec([V(f'{v.lane(0)}*{m.lane(0)}+{v.lane(1)}*{m.lane(1)}'),V(f'{v.lane(0)}*{m.lane(2)}+{v.lane(1)}*{m.lane(3)}')])
 name={'==':'equal','!=':'different','<':'less','>':'greater','<=':'less_equal','>=':'greater_equal','&&':'logical_and','||':'logical_or','%':'mod','&':'bit_and','|':'bit_or','^':'bit_xor','>>':'shift_right','<<':'shift_left'}.get(op)
 v=V(f'{name}({a}, {b})' if name else f'({a} {op} {b})',1 if name and op not in ['%','&','|','^','>>','<<'] else max(a.w,b.w))
 if a.constant is not None and b.constant is not None:
  x,y=a.constant,b.constant
  try:v.constant={'+':lambda:x+y,'-':lambda:x-y,'*':lambda:x*y,'/':lambda:x/y,'<':lambda:float(x<y),'>':lambda:float(x>y),'==':lambda:float(x==y),'!=':lambda:float(x!=y),'<=':lambda:float(x<=y),'>=':lambda:float(x>=y)}[op]()
  except:pass
 return v
class Compiler:
 def __init__(self,env):self.env=env;self.lines=[];self.tokens=[];self.i=0;self.scopes=[];self.functions={};self.function_depth=0;self.definitions={}
 def take(self,s=None):
  if self.i>=len(self.tokens):raise ValueError('Unexpected end of shader')
  t=self.tokens[self.i]
  if s is not None and t!=s:raise ValueError(f'expected {s} got {t}')
  self.i+=1;return t
 def peek(self):return self.tokens[self.i] if self.i<len(self.tokens) else ''
 def compact(self,v):
  if len(v.s)>50:
   name='v'+str(len(self.lines));self.lines.append(f'let {name} = {v}');self.definitions[name]=v;out=V(name,v.w,v.mat);out.constant=v.constant;out.origin=v.origin;return out
  return v
 def shifted(self,value,delta):
  cache={}
  def text(expr):
   def node(m):
    name=m[0]
    if name not in self.definitions:return name
    if name not in cache:
     old=self.definitions[name];rhs=text(old.s);new='v'+str(len(self.lines));self.lines.append(f'let {new} = {rhs}');cache[name]=new;self.definitions[new]=V(rhs,old.w,old.mat)
    return cache[name]
   # Original SSA dependencies are cloned once at the shifted coordinate.
   expr=re.sub(r'\bv\d+\b',node,expr)
   return re.sub(r'\buv\b','(uv + '+delta+')',expr)
  return self.compact(V(text(value.s),value.w,value.mat))
 def expr(self,minp=0):
  t=self.take();ref=t if t in self.env else None
  if t in ['-','+','!']:
   a=self.expr(9);v=V(f'(-{a})',a.w) if t=='-' else binary('==',a,V('0')) if t=='!' else a
   if t=='-' and a.constant is not None:v.constant=-a.constant
  elif t=='(':
   v=self.expr();self.take(')')
  elif re.match(r'^(\d|\.)',t):v=V(str(int(t.rstrip('uU'),16)) if t.lower().startswith('0x') else re.sub('[fFuU]$','',t))
  elif t in ['true','false']:v=V('1' if t=='true' else '0')
  elif self.peek()=='(':
   self.take('(');args=[]
   while self.peek()!=')':
    args.append(self.expr())
    if self.peek()!=',':break
    self.take(',')
   self.take(')')
   if t in ['vec2','vec3','vec4','mat2']:
    n=TYPES[t]
    if len(args)==1 and args[0].w>n:args=[V(f'({args[0]}).'+('xyzw'[:n]),n)]
    v=vec(args,n);v.mat=t=='mat2'
   elif t in ['dFdx','dFdy','fwidth']:
    a=args[0];dx=binary('-',self.shifted(a,'vec2(aa, 0)'),a);dy=binary('-',self.shifted(a,'vec2(0, aa)'),a)
    v=dx if t=='dFdx' else dy if t=='dFdy' else V(f'(abs({dx}) + abs({dy}))',a.w)
   elif t in ['float','int','bool']:v=args[0] if t=='float' else V(f'trunc({args[0]})',args[0].w)
   elif t in self.functions:
    formal,body,ret=self.functions[t]
    if len(args)!=len(formal):raise ValueError('function args '+t)
    if self.function_depth>=12:raise ValueError('recursive helper '+t)
    old=self.env.copy();self.env.update(zip(formal,args))
    if ret is not None:ret,mat=ret
    else:mat=False
    if ret is None:
     savedt,savedi=self.tokens,self.i;self.tokens=TOK.findall(body.removeprefix('return ').removesuffix(';'));self.i=0;v=self.expr();self.tokens,self.i=savedt,savedi;self.env=old
    else:
     self.env['__return']=V('0') if ret==1 else vec([V('0')],ret);self.env['__returned']=V('0');self.env['__return'].mat=mat
    if ret is not None:
     self.function_depth+=1;self.execute(TOK.findall(body));v=self.env['__return'];self.function_depth-=1;self.env=old
   elif t=='InputFunc':
    args=args+[V('vec3(0)',3)]*(7-len(args));v=V(f'(sample_context({args[1]}, {args[0]}, {args[2]}, {args[3]}, {args[4]}, {args[5]}, {args[6]})).rgb',3)
   else:v=V(f'{t}('+', '.join(str(a) for a in args)+')',1 if t in ['dot','length','distance'] else max(a.w for a in args))
  else:
   if t not in self.env:raise ValueError('unknown '+t)
   v=self.env[t]
  v=self.compact(v)
  while True:
   t=self.peek()
   if t in ['++','--']:
    self.take()
    if not ref:raise ValueError('increment target')
    self.assign(ref,binary('+' if t=='++' else '-',v,V('1')));continue
   if t=='.':
    self.take();sw=self.take();v=V(f'component({v}, {self.env[sw]})',1) if sw in self.env and not all(c in 'xyzwrgba' for c in sw) else V(f'({v}).{sw}',len(sw));continue
   if t=='[':
    self.take();index=self.expr();self.take(']')
    if v.mat:v=V(f'select(equal({index}, 0), ({v}).xy, ({v}).zw)',2)
    else:v=V(f'component({v}, {index})',1)
    continue
   if t=='?' and minp==0:
    self.take();a=self.expr();self.take(':');b=self.expr();w=max(a.w,b.w)
    if a.w!=w:a=vec([a],w)
    if b.w!=w:b=vec([b],w)
    v=V(f'select({v}, {a}, {b})',w);continue
   p={'||':1,'&&':2,'&':3,'|':3,'^':3,'>>':4,'<<':4,'==':3,'!=':3,'<':4,'>':4,'<=':4,'>=':4,'+':5,'-':5,'*':6,'/':6,'%':6}.get(t,0)
   if p==0 or p<minp:break
   self.take();v=self.compact(binary(t,v,self.expr(p+1)))
  return v
 def assign(self,name,v,sw=None):
  if len(self.lines)>4096:raise ValueError('Needs structured sampling primitive')
  if sw:
   old=self.env[name];lanes=[V(old.lane(i)) for i in range(old.w)]
   for j,c in enumerate(sw):lanes['xyzw'.index(c.translate(str.maketrans('rgba','xyzw')))]=V(v.lane(j))
   v=vec(lanes)
  n='v'+str(len(self.lines));self.lines.append(f'let {n} = {v}');self.definitions[n]=v;self.env[name]=V(n,v.w,v.mat);self.env[name].constant=v.constant;self.env[name].origin=v.origin
 def simple_statement(self):
  if self.peek() in ['for','if','while','{','#','<!']:raise ValueError('control '+self.peek())
  # Historical expression statements are intentionally preserved as no-ops.
  if self.peek() in self.env and self.tokens[self.i+1] not in ['=','+=','-=','*=','/=','.', '++','--',',',';']:
   self.expr();self.take(';');return
  if self.peek()=='const':self.take()
  typ=None
  if self.peek() in TYPES:typ=self.take()
  while True:
   name=self.take();sw=None
   if typ and self.scopes and name not in self.scopes[-1]:self.scopes[-1][name]=self.env.get(name)
   if not re.match(r'^\w+$',name):raise ValueError('invalid lhs '+name)
   if self.peek()=='.':self.take();sw=self.take()
   op=self.take()
   if op in ['++','--']:
    self.assign(name,binary('+' if op=='++' else '-',self.env[name],V('1')));op=self.take()
   elif op in [',',';']:

    v=V('0',1) if TYPES[typ]==1 else vec([V('0')],TYPES[typ]);self.assign(name,v)
   else:
    if op not in ['=','+=','-=','*=','/=']:raise ValueError('assign '+op)
    v=self.expr()
    if op!='=':
     old=self.env[name]
     if sw:old=V(f'({old}).{sw}',len(sw))
     v=binary(op[0],old,v)
    self.assign(name,v,sw);op=self.take()
   if op==';':return
   if op!=',':raise ValueError('statement end '+op)
 def execute(self,tokens):
  oldt,oldi=self.tokens,self.i;self.tokens=tokens;self.i=0
  while self.i<len(self.tokens):self.statement()
  self.tokens,self.i=oldt,oldi
 def capture(self):
  start=self.i
  def group(op,cl):
   self.take(op);depth=1
   while depth:
    t=self.take();depth+=(t==op)-(t==cl)
  t=self.peek()
  if t=='{':group('{','}')
  elif t in ['if','for','while']:
   self.take();group('(',')');self.capture()
   if t=='if' and self.peek()=='else':self.take();self.capture()
  else:
   depth=0
   while True:
    t=self.take();depth+=(t=='(')-(t==')')
    if t==';' and depth==0:break
    if not t:raise ValueError('Unterminated statement')
  return self.tokens[start:self.i]
 def merge(self,condition,before,yes,no):
  self.env=before.copy()
  for name,old in before.items():
   a=yes.get(name,old);b=no.get(name,old)
   if a.origin!=b.origin:
    if a.w!=b.w:raise ValueError('branch width '+name)
    self.assign(name,V(f'select({condition}, {a}, {b})',a.w,a.mat))
 def statement(self):
  if self.peek()==';':self.take();return
  if self.peek()=='return':
   self.take();v=self.expr();self.take(';')
   old=self.env['__return'];flag=self.env['__returned']
   if v.w!=old.w:raise ValueError('return width')
   self.assign('__return',V(f'select({flag}, {old}, {v})',v.w,v.mat));self.assign('__returned',V('1'));return
  if self.peek()=='{':
   self.take();self.scopes.append({})
   while self.peek()!='}':self.statement()
   self.take('}')
   for name,old in self.scopes.pop().items():
    if old is None:self.env.pop(name,None)
    else:self.env[name]=old
   return
  if self.peek()=='if':
   self.take();self.take('(');c=self.expr();self.take(')')
   before=self.env.copy();self.statement();yes=self.env.copy();self.env=before.copy()
   if self.peek()=='else':self.take();self.statement()
   no=self.env.copy();self.merge(c,before,yes,no);return
  if self.peek()=='for':
   self.take();self.take('(');before_loop=self.env.copy();self.statement()
   start=self.i
   while self.peek()!=';':self.take()
   cond=self.tokens[start:self.i];self.take(';');start=self.i;depth=0
   while self.peek()!=')' or depth:
    t=self.take();depth+=(t=='(')-(t==')')
   inc=self.tokens[start:self.i];self.take(')');body=self.capture()
   if len(inc)==2 and inc[0] in ['++','--']:inc=[inc[1],inc[0]]
   if len(inc)==2 and inc[1] in ['++','--']:inc=[inc[0],'+=' if inc[1]=='++' else '-=','1',';']
   else:inc=inc+[';']
   if len(cond)<3 or cond[1] not in ['<','<=','>','>=']:raise ValueError('Noncanonical loop')
   iterator=cond[0];initial=self.env[iterator]
   savedt,savedi=self.tokens,self.i;self.tokens=cond[2:];self.i=0;limit=self.expr();self.tokens,self.i=savedt,savedi
   # Derive the runtime iteration count from the original bound and increment.
   sep=inc.index(';') if ';' in inc else len(inc);first=inc[:sep]
   try:comma=first.index(',');first=first[:comma]
   except ValueError:pass
   if len(first)==2 and first[1] in ['++','--']:first=[first[0],'+=' if first[1]=='++' else '-=','1']
   if first[0]!=iterator or first[1] not in ['+=','-=']:raise ValueError('Loop increment')
   savedt,savedi=self.tokens,self.i;self.tokens=first[2:];self.i=0;step=self.expr();self.tokens,self.i=savedt,savedi
   if first[1]=='-=':step=binary('-',V('0'),step)
   count=binary('/',binary('-',limit,initial),step)
   if cond[1] in ['<=','>=']:count=binary('+',count,V('1'))
   hard=1024
   if count.constant is not None:hard=max(1,int(__import__('math').ceil(count.constant)))
   elif len(cond)==3 and cond[2] in self.ranges:hard=max(1,int(self.ranges[cond[2]])+1)
   elif any('shapeN' in t for t in cond) or 'N' in cond:hard=64
   if hard>1024:raise ValueError('Loop maximum exceeds 1024')
   # Carry source variables through a bounded loop; temporaries remain SSA.
   before=self.env.copy();states={}
   for name,old in before.items():
    state='v'+str(len(self.lines));self.lines.append(f'state {state} = {old}')
    states[name]=V(state,old.w,old.mat);states[name].origin=old.origin;self.env[name]=states[name]
   savedt,savedi=self.tokens,self.i;self.tokens=cond;self.i=0;c=self.expr();self.tokens,self.i=savedt,savedi
   self.lines.append(f'repeat {hard} times {count} while {c}')
   self.execute(body);self.execute(inc)
   for name,state in states.items():
    value=self.env.get(name,state)
    if value.origin!=state.origin:self.lines.append(f'update {state} = {value}')
   self.lines.append('end repeat')
   self.env={name:(V(states[name].s,states[name].w,states[name].mat) if self.env.get(name,states[name]).origin!=before[name].origin else before[name]) for name in before_loop}
   return
  self.simple_statement()
 def run(self,body):
  self.tokens=TOK.findall(body);self.i=0
  while self.i<len(self.tokens):self.statement()
  out=self.env.get('outColor')
  if not out:raise ValueError('no outColor')
  if out.w==1:out=vec([out],3)
  if out.w==3:out=vec([out,V('input.a')],4)
  self.lines.append('image = '+str(out));return '\n'.join(self.lines)+'\n'
def metadata(source):
 lines=[];params=[];name=category='';body=[];terms=[];desc=''
 for line in source.splitlines():
  m=re.match(r'\s*(\w+)\s*=\s*(.*?);?\s*$',line)
  if m and (m[1] in ['outName','outCategory','inTerminal','inDefaultTerminal','ParameterGroup'] or m[1].startswith('Param')):
   key=m[1];q=re.findall(r'"([^"\n]*)"|\x27([^\x27\n]*)\x27',m[2]);q=[a or b for a,b in q]
   if key=='outName':name=q[0]
   elif key=='outCategory':category=q[0]
   elif key=='inTerminal':terms.append(q)
   elif key=='ParameterGroup':lines.append('group '+json.dumps(q[0])+(' false' if len(q)>1 and q[1]=='false' else ' true'))
   elif key in ['ParamDescription','ParamHtml']:desc=q[0]
   elif key=='ParamToolTip':lines.append('tooltip '+json.dumps(q[0]))
   elif key=='ParamList':
    opts=[v.strip() for v in q[3].split(',')]
    lines.append(f'param float {q[0]} {json.dumps(q[1])} default {q[2]} range 0 {len(opts)-1}')
    lines.append('choices '+' '.join(json.dumps(v) for v in opts));params.append((q[0],'float',1))
   elif key in ['ParamSlider','ParamVector2','ParamVector3','ParamBoolean','ParamColor']:
    typ={'ParamSlider':'float','ParamVector2':'vec2','ParamVector3':'vec3','ParamBoolean':'bool','ParamColor':'color'}[key];w={'float':1,'vec2':2,'vec3':3,'bool':1,'color':4}[typ]
    default=q[2:2+w] if typ!='color' else re.findall(r'[-+]?(?:\d*\.\d+|\d+\.?\d*)',q[2].split('(',1)[1])
    params.append((q[0],typ,w));decl=f'param {typ} {q[0]} {json.dumps(q[1])} default '+' '.join(default)
    if typ not in ['bool','color']:decl+=' range '+' '.join(q[2+w:4+w])
    lines.append(decl)
  else:body.append(line)
 return name,category,desc,lines,params,terms,'\n'.join(body)
def preprocess(body):
 funcs={}
 body=body.replace('\\\n','')
 body=re.sub(r'<!\s*(?!FUNC)(\w+)\s*!>',lambda m:'fn_'+m[1],body)
 objects={m[1]:m[2].strip() for m in re.finditer(r'^\s*#\s*define\s+(\w+)\s+([^\n]+)',body,re.M)}
 active=[True];branches=[];kept=[]
 for line in body.splitlines():
  m=re.match(r'\s*#\s*(ifdef|ifndef|if|else|endif)\b\s*(.*)',line)
  if m:
   if m[1] in ['if','ifdef','ifndef']:
    value=objects.get(m[2].strip(),m[2].strip());yes=(m[2].strip() in objects) if m[1]=='ifdef' else (m[2].strip() not in objects) if m[1]=='ifndef' else value!='0';branches.append(yes);active.append(active[-1] and yes)
   elif m[1]=='else':active[-1]=active[-2] and not branches[-1]
   else:active.pop();branches.pop()
  elif active[-1]:kept.append(line)
 body='\n'.join(kept)
 body=re.sub(r'^\s*#\s*define\s+(\w+)\s+([^\n]+)','',body,flags=re.M)

 # Function macros expand through the same typed inliner as regular helpers.
 def macro(m):
  name,args,expr=m[1],m[2],m[3]
  funcs[name]=(args.split(',') if args.strip() else [],'return '+expr+';',None)
  return ''
 body=re.sub(r'^\s*#define\s+(\w+)\(([^)]*)\)\s*([^\n]+)',macro,body,flags=re.M)
 def function(m):
  block=m[1];f=re.search(r'(float|vec[234]|mat2|void)\s+(\w+)\s*\((.*?)\)\s*\{(.*)\}',block,re.S)
  if not f:raise ValueError('helper declaration '+block[:60])
  formals=[re.sub(r'\b(in|out|inout|const)\b','',x).strip().split()[-1] for x in f[3].split(',') if x.strip()]
  code=f[4]
  rettype=f[1]
  if rettype=='void' and 'inout vec2' in f[3]:rettype='vec2';code+=' return '+formals[0]+';'
  if 'switch' in code:
   case=re.findall(r'case\s+(\d+)\s*:(.*?)(?=case|$)',code.split('{',1)[1].rsplit('}',1)[0],re.S)
   code=''.join(('if' if i==0 else 'else if')+'('+formals[0]+'=='+n+'){'+c+'}' for i,(n,c) in enumerate(case))
  if ('fn_scramble' in code or '>>8U' in code) and f[1]=='vec3' and len(formals)==1:code='return hash33('+formals[0]+');'
  if f[2]=='fn_profile' and 'mat4x2' in code:
   u,t=formals;terms=[]
   for i in ['x','y']:
    a=f'{u}.{i}';k=f'{t}.{i}';terms.append(f'select(equal({k},0),{a},select(equal({k},1),1.-{a},select(equal({k},2),2.*min({a},1.-{a}),.5-.5*cos(6.28319*{a}))))')
   code='return vec2('+', '.join(terms)+');'
  code=re.sub(r'return\s+(\w+)\s*([+*/-])=\s*(.*?);',r'\1 \2= \3; return \1;',code,flags=re.S)
  funcs[f[2]]=(formals,code,(TYPES[rettype],rettype=='mat2'))
  return ''
 body=re.sub(r'<!\s*FUNC\s+\w+\s*!>(.*?)<!\s*FUNC END\s*!>',function,body,flags=re.S)
 for key,expr in objects.items():
  if key=='fn_scramble':continue
  body=re.sub(r'\b'+key+r'\b','('+expr+')',body)
  funcs={name:(args,re.sub(r'\b'+key+r'\b','('+expr+')',code),ret) for name,(args,code,ret) in funcs.items()}
 # Preserve signed-coordinate hash conversion without rounding a uint into float.
 if 'fn_unsigned' in funcs:funcs['fn_unsigned']=(['v'],'return select(greater_equal(v,0),v,v-1);',None)
 body=re.sub(r'fn_pR\((\w+),([^;]*)\);',r'\1 = fn_pR(\1,\2);',body)
 funcs={name:(args,re.sub(r'fn_pR\((\w+),([^;]*)\);',r'\1 = fn_pR(\1,\2);',code),ret) for name,(args,code,ret) in funcs.items()}
 return body,funcs
def aperiodic():
 # Literal port of the five-dimensional plane projection and preferred-face search.
 u=[.632456,.19544,-.511667,-.511667,.19544];v=[0.,.601501,.371748,-.371748,-.601501]
 u2=[.632456,-.511667,.19544,.19544,-.511667];v2=[0.,.371748,-.601501,.601501,-.371748]
 out=[]
 def put(expr):
  name='v'+str(len(out));out.append('let '+name+' = '+expr);return name
 U=put('VCELLST * (uv - vec2(0.5))')
 cpo=[put(f'CP_tans_u2dir * {u2[k]} + CP_tans_v2dir * {v2[k]} + CP_tans_adir * 0.447214') for k in range(5)]
 p=[put(f'{cpo[k]} + {U}.x * {u[k]} + {U}.y * {v[k]}') for k in range(5)]
 ip=[put(f'floor({p[k]} + 0.5)') for k in range(5)]
 direction=[put(f'2 * greater({p[k]} - {ip[k]}, 0) - 1') for k in range(5)]
 found=put('0');chosen=put('0');face=put('0')
 for i in range(4):
  for j in range(i+1,5):
   dot=put(f'{u[i]} * {u[j]} + {v[i]} * {v[j]}')
   f=put(f'sqrt({u[i]} * {u[i]} + {v[i]} * {v[i]} - {dot} * {dot} / ({u[j]} * {u[j]} + {v[j]} * {v[j]}))')
   determinant=put(f'{u[i]} * {v[j]} - {u[j]} * {v[i]}')
   a,b,c,d=[put(f'{x} / {determinant}') for x in [v[j],-v[i],-u[j],u[i]]]
   lhs=put(f'vec2({ip[i]} - {cpo[i]} + 0.5 * {direction[i]}, {ip[j]} - {cpo[j]} + 0.5 * {direction[j]})')
   z=put(f'vec2({lhs}.x * {a} + {lhs}.y * {b}, {lhs}.x * {c} + {lhs}.y * {d})')
   q=[put(f'floor({cpo[k]} + {z}.x * {u[k]} + {z}.y * {v[k]} + 0.5)') for k in range(5)]
   offsets=[put(f'{p[k]} - ('+(f'{ip[k]} + 0.5 * ({direction[k]} - 1)' if k in [i,j] else q[k])+')') for k in range(5)]
   px=put(' + '.join(f'{offsets[k]} * {u[k]}' for k in range(5)))
   py=put(' + '.join(f'{offsets[k]} * {v[k]}' for k in range(5)))
   pit=put(f'vec2({a} * {px} + {c} * {py}, {b} * {px} + {d} * {py})')
   plane=put(f'abs({pit} - vec2(0.5)) - vec2(0.5)')
   dist=put(f'max({plane}.x, {plane}.y) * {f}')
   match=put(f'logical_and(less({dist}, 0), equal({found}, 0))')
   chosen=put(f'select({match}, {dist}, {chosen})');face=put(f'select({match}, abs({determinant}) * 2.5, {face})');found=put(f'max({found}, {match})')
 distance=put(f'clamp((abs({chosen}) - GAP / 2) / nonzero(BEVEL), 0, 1)')
 ident=put(f'select(equal({distance}, 0), 0, select(less({face}, 0.7), 0, 1))')
 return '\n'.join(out)+f'\nimage = vec4({ident}, {distance}, 0, input.a)\n'
def main():
 converted=[];failed={};skip={'noise_briansperlin2d.fs','shape2d_disc.fs','op_multiply.fs','op_colorblend.fs','trans_scale_offset.fs'}
 manifest=json.loads((ROOT/'shaders/library.json').read_text())
 maintained=[x for x in manifest if x.get('authored')]
 stable_ids={x['file']:x.get('id',x['stem']) for x in manifest}
 allowed={x['file'] for x in manifest if not x.get('authored')}
 allfiles=list((ROOT/'shaders/original').glob('*.fs'));names={}
 for p in allfiles:
  n,c,*_=metadata(p.read_text());names[n]=names.get(n,0)+1
 for p in sorted(allfiles):
  if p.name not in allowed or p.name in skip or p.name.startswith('x') or p.name.startswith('matmixer'):continue
  source=p.read_text()
  try:
   name,category,desc,lines,params,terms,body=metadata(source)
   desc=re.sub(r'<[^>]+>',' ',desc);desc=' '.join(desc.split())
   if names[name]>1 or name in ['Color','Circle','Border']:name+=' ('+category+')'
   env={'outColor':V('vec3(0)',3),'inResolution':V('vec2(256)',2),'inUV':V('uv',2),'inPos':V('pos',3),'inNormal':V('normal',3),'inInput1':V('input.rgb',3),'inInput2':V('vec3(0)',3),'inInput3':V('vec3(0)',3),'inParam':V('vec3(0)',3)}
   if '3D' in category:
    lines.append('group "Sampling" true')
    lines.append('param float slice "Z Slice" default 0 range -1000 1000');env['inPos']=V('(pos + vec3(0, 0, slice))',3)
   if category=='Shape':env['inInput1']=V('select(aux_valid, aux1, vec3(0))',3)
   for id,typ,w in params:env[id]=V(id+'.rgb' if typ=='color' else id,3 if typ=='color' else w)
   if '3D' not in category and any(re.search(r'\b'+v+r'\b',body) for v in ['inTime','inParam','inInput2','inInput3']):lines.append('group "Sampling" true')
   for token,id,label,w in [('inTime','time','Time Offset',1),('inParam','utility','Utility Parameter',3),('inInput2','extra2','Input 2',3),('inInput3','extra3','Input 3',3)]:
    if re.search(r'\b'+token+r'\b',body):
     default='0' if w==1 else '0 0 0'
     term=next((t for t in terms if t[0]==('1' if id=='extra2' else '2')),None) if id in ['extra2','extra3'] else None
     if term:
      nums=re.findall(r'[-+]?(?:\d*\.\d+|\d+\.?\d*)',term[2].split('(',1)[1]);default=' '.join(nums*3 if len(nums)==1 else nums)
     lines.append(f'param {"float" if w==1 else "vec3"} {id} {json.dumps(label)} default {default}')
     env[token]=V(f'select(aux_valid, {"aux2" if id=="extra2" else "aux3"}, {id})',w) if id in ['extra2','extra3'] else V(f'(sample_time + {id})',1) if id=='time' else V(f'select(aux_valid, aux3, {id})',3) if id=='utility' else V(id,w)
   body=re.sub(r'/\*.*?\*/','',body,flags=re.S);body=re.sub(r'//[^\n]*','',body)
   body=re.sub(r'(?:(?:const\s+)?uint\s+)?k\s*=\s*1103515245U\s*;', '',body)
   hashpat=r'(uvec3\s+)?(\w+)\s*=\s*uvec3\(([^;]*?)\);\s*((?:\2\s*=\s*\(\(\2>>8U\)\^\2\.yzx\)\*k;\s*){3})'
   body=re.sub(hashpat,lambda m:('vec3 ' if m[1] else '')+m[2]+' = hash33(vec3('+m[3]+'));',body,flags=re.S)
   body=re.sub(r'vec3\((\w+)\)\s*\*\s*\(1\.0/float\(0xffffffffU\)\)',r'\1',body)
   body=body.replace('float(0xffffffffU)','4294967295.0')
   body=re.sub(r'\b[ui]vec([234])\b',r'vec\1',body)
   if p.name=='pattern_cutandproject.fs':body=""
   body,functions=preprocess(body)
   functions={name:(args,re.sub(r'\b[ui]vec([234])\b',r'vec\1',code),ret) for name,(args,code,ret) in functions.items()}
   comp=Compiler(env);comp.functions=functions;comp.ranges={}
   for line in lines:
    m=re.match(r'param \w+ (\w+) .* range [^ ]+ ([^ ]+)$',line)
    if m:comp.ranges[m[1]]=float(m[2])
   code=aperiodic() if p.name=='pattern_cutandproject.fs' else comp.run(body)
   out='tile 1\nname '+json.dumps(name)+'\ncategory '+json.dumps(category)+'\ndescription '+json.dumps(desc)+'\n'+'\n'.join(lines)+'\n\n'+code
   stem=p.stem;(ROOT/'shaders/tiles'/f'{stem}.tile').write_text(out)
   original=''.join(l+'\n' for l in source.splitlines() if not l.startswith('ParamReferences'))
   (ROOT/'shaders/original'/p.name).write_text(original)
   converted.append({'stem':stem,'file':p.name,'name':name,'category':category,'input':any(t[0]=='0' for t in terms),'id':stable_ids[p.name]})
  except Exception as e:
   failed[p.name]=str(e)
 (ROOT/'shaders/library.json').write_text(json.dumps(converted+maintained,indent=2));(ROOT/'work/port-pending.json').write_text(json.dumps(failed,indent=2))
 print('Converted',len(converted),'Pending',len(failed));print(json.dumps(failed,indent=2))
if __name__=='__main__':main()
