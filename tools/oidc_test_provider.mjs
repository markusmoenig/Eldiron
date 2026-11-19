// Task-owned loopback identity provider for protocol/browser acceptance tests.
// Generates real RSA signatures; it is never a production account service.
import http from 'node:http';
import fs from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {generateKeyPairSync,sign} from 'node:crypto';
export async function startIdentityProvider(directory){
    const {privateKey,publicKey}=generateKeyPairSync('rsa',{modulusLength:2048});
    const jwk={...publicKey.export({format:'jwk'}),kid:'browser-test-key',alg:'RS256',use:'sig'};
    const encode=value=>Buffer.from(JSON.stringify(value)).toString('base64url');
    const state={issuer:'',nonce:'',approved:false,tokenRequests:0,errors:[]};
    const server=http.createServer(async(req,res)=>{
        try {
            let data='';for await(const chunk of req){data+=chunk;assert(data.length<65536);}
            const form=new URLSearchParams(data),route=new URL(req.url,'http://localhost').pathname;
            res.setHeader('Content-Type','application/json');res.setHeader('Cache-Control','no-store');
            if(route==='/.well-known/openid-configuration')return res.end(JSON.stringify({issuer:state.issuer,device_authorization_endpoint:state.issuer+'/device',token_endpoint:state.issuer+'/token',jwks_uri:state.issuer+'/keys'}));
            if(route==='/keys')return res.end(JSON.stringify({keys:[jwk]}));
            if(route==='/device'){
                assert.equal(form.get('client_id'),'browser-client');assert.equal(form.get('scope'),'openid');
                state.nonce=form.get('nonce');assert.equal(state.nonce.length,64);state.approved=false;
                return res.end(JSON.stringify({device_code:'PRIVATE_DEVICE_CODE',user_code:'TEST-CODE',verification_uri:state.issuer+'/verify',expires_in:300,interval:1}));
            }
            if(route==='/verify'){
                res.setHeader('Content-Type','text/html');
                if(req.method==='POST'&&form.get('code')==='TEST-CODE'){state.approved=true;return res.end('<p>Test provider authorization approved</p>');}
                return res.end('<form method="post"><input name="code" value="TEST-CODE"><button id="approve">Approve test identity</button></form>');
            }
            if(route==='/token'){
                state.tokenRequests++;assert.equal(form.get('device_code'),'PRIVATE_DEVICE_CODE');
                assert.equal(form.get('grant_type'),'urn:ietf:params:oauth:grant-type:device_code');
                if(!state.approved){res.statusCode=400;return res.end(JSON.stringify({error:'authorization_pending'}));}
                const now=Math.floor(Date.now()/1000),payload=encode({alg:'RS256',kid:jwk.kid,typ:'JWT'})+'.'+encode({iss:state.issuer,sub:'stable-browser-subject',aud:'browser-client',nonce:state.nonce,iat:now,exp:now+300});
                return res.end(JSON.stringify({id_token:payload+'.'+sign('RSA-SHA256',Buffer.from(payload),privateKey).toString('base64url')}));
            }
            res.statusCode=404;res.end('{}');
        } catch(error){state.errors.push(String(error));res.statusCode=500;res.end('{}');}
    });
    await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
    state.issuer=`http://127.0.0.1:${server.address().port}`;
    const config=path.join(directory,'private-identity.toml');
    await fs.writeFile(config,`[[providers]]\nid="test"\nlabel="Test identity provider"\nissuer="${state.issuer}"\nclient_id="browser-client"\nallow_insecure_loopback=true\n`,{mode:0o600});
    return {state,config,close:()=>new Promise(resolve=>server.close(resolve))};
}
