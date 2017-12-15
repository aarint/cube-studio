import {clipboard, remote} from 'electron';

let currentWindow = remote.getCurrentWindow();
console.log(currentWindow); 