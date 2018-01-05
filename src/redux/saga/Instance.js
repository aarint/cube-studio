import { put, call } from "redux-saga/effects";
import { addInstance, getInstance, getSavedInstancesDone } from "../actions/Instance";
import { getSavedInstances } from '../../utils/InstanceUtil';

export function* addConnectedInstance(action) {
    try {
        //yield put(addInstance(action.instance));
        //addNewInstance(action.key, action.instance);
    } catch (ex) {
        console.log('Wrong with adding aninstance');
    }
}

export function* getAllSavedInstances() {
    try {
        const instances = getSavedInstances();
        yield put(getSavedInstancesDone(instances));
    } catch (ex) {
        console.log(ex);
    }
}

export function* getConnectedInstance(key) {
    try {
    } catch (ex) {
        console.log("Wrong with getting an instance.");
        return null;
    }
}

export function* deleteConnectedInstance(key) {
    try {
    } catch (ex) {
        console.log("Wrong with deleting an instance.");
    }
}

export function* getConnectedConfig(key) {
    console.log('get current connnected instance config!!!');
}