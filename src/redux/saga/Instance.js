import {setItem,getItem,removeItem} from '../../utils/StorageUtil';

export function* addInstance(key,instance){
    try{
        setItem(key,instance);
    }catch(ex){
        console.log('Wrong with adding aninstance');
    }
}

export function* getAllInstances(){
    try{
        return getItem(key);
    }catch(ex){
        console.log("Wrong with getting all instances.");
        return null;
    }
}

export function* getInstance(key){
    try{
        return getItem(key);
    }catch(ex){
        console.log("Wrong with getting an instance.");
        return null;
    }
}

export function* deleteInstance(key){
    try{
        removeItem(key);
    }catch(ex){
        console.log("Wrong with deleting an instance.");
    }
}

export function* getConfig(){}