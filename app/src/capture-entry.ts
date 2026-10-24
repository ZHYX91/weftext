import "./style.css";
import "./data-view.css";
import "./capture.css";
import { api } from "./api";
import { mountLocalCapture } from "./capture-mount";
import { InterfaceLocaleControl, localLocalePreferences } from "./ui-locale-settings";
import { errorUiText, setUiText, message, locale } from "./ui-locale";
import { NativeSurface } from "./native-host";
import { NativeCapture } from "./native-capture";
const nativeSurface=window.chrome?.webview?new NativeSurface("capture"):undefined;
const host=document.getElementById("capture");
if(!host)throw new Error("Capture root is unavailable");
const language=new InterfaceLocaleControl(localLocalePreferences(api));host.before(language.element);
const title=document.querySelector("title");if(title)setUiText(title, message("capture.title"));
const updateLanguage=()=>document.documentElement.lang=locale.current;updateLanguage();const releaseLanguage=locale.subscribe(updateLanguage);
window.addEventListener("pagehide",event=>{if(event.persisted)return;language.dispose();releaseLanguage();});
const query=new URLSearchParams(location.search),draftId=query.get("draftId")??undefined;
if([...query.keys()].some(key=>key!=="draftId") || draftId && !/^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/.test(draftId))throw new Error("Capture surface address is invalid");
let native:NativeCapture|undefined;
void mountLocalCapture(host,{request:api,options:{draftId,pointerStorage:localStorage},
  ...(nativeSurface?{deliverCommitted:(original:import("./capture-local").CaptureResultIdentity)=>{
    if(!native)throw new Error("The original native Capture owner is not ready");
    return native.open(original);
  },requestClose:()=>{
    if(!native)throw new Error("The original native Capture owner is not ready");
    return native.requestClose();
  }}:{})}).then(mounted=>{
    if(!nativeSurface)return;
    native=new NativeCapture(nativeSurface,mounted);
    nativeSurface.register("capture",native.participant);
    nativeSurface.markReady();
  }).catch(error=>{setUiText(host,errorUiText(error));nativeSurface?.reportFailed();});
