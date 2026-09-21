const status = document.getElementById('status');
const decode = value => Uint8Array.from(atob(value.replace(/-/g,'+').replace(/_/g,'/')), c => c.charCodeAt(0));
const encode = value => btoa(String.fromCharCode(...new Uint8Array(value))).replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'');
const post = (path, body) => fetch(path, {method:'POST',credentials:'same-origin',headers:{'content-type':'application/json','x-csrf-token':csrf},body:JSON.stringify(body)});
const wire = credential => ({id:credential.id,raw_id:encode(credential.rawId),type:credential.type,response:{client_data_json:encode(credential.response.clientDataJSON)}});
let previous;
document.getElementById('register').onclick = async () => { try {
  const start = await post('/a/register',{}); if(!start.ok) throw Error('registration start');
  const {options,expected} = await start.json(); const p = options.publicKey;
  const credential = await navigator.credentials.create({publicKey:{challenge:decode(p.challenge),rp:p.rp,user:{id:decode(p.user.id),name:p.user.name,displayName:p.user.display_name},pubKeyCredParams:p.pubKeyCredParams,timeout:p.timeout,attestation:p.attestation,authenticatorSelection:{residentKey:p.authenticatorSelection.resident_key,userVerification:p.authenticatorSelection.user_verification}}});
  const data=wire(credential); data.response.attestation_object=encode(credential.response.attestationObject);
  const result=await post('/b/register',{expected,credential:data}); if(result.status!==204) throw Error('registration finish');
  status.textContent='registered';
} catch(error) {status.textContent='error: '+error.message;} };
document.getElementById('authenticate').onclick = async () => { try {
  const start=await post('/a/authenticate',{}); if(!start.ok) throw Error('authentication start');
  const {options,expected}=await start.json(); const p=options.publicKey;
  const credential=await navigator.credentials.get({publicKey:{challenge:decode(p.challenge),rpId:p.rp_id,allowCredentials:p.allow_credentials.map(c=>({type:c.type,id:decode(c.id)})),timeout:p.timeout,userVerification:p.user_verification}});
  const data=wire(credential); data.response.authenticator_data=encode(credential.response.authenticatorData); data.response.signature=encode(credential.response.signature);
  previous={expected,credential:data,user_handle:credential.response.userHandle?encode(credential.response.userHandle):null};
  const result=await post('/b/authenticate',previous); if(result.status!==204) throw Error('authentication finish');
  status.textContent='authenticated';
} catch(error) {status.textContent='error: '+error.message;} };
document.getElementById('replay').onclick=async()=>{const result=await post('/b/authenticate',previous);status.textContent=result.status===401?'replay-rejected':'error: replay accepted';};
