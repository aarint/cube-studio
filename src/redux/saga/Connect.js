import { put, call } from 'redux-saga/effects';
import Redis from 'redis';
import { connectDBStart, connectDBDone, connectDBError } from '../actions/Connect';
import {addNewInstance} from "../../utils/InstanceUtil";

export function* connectDBInstance(name) {
    try {
        yield put(connectDBStart());

        let client = Redis.createClient(6379, '10.2.1.128');

        client.on('connect',  () => {
            console.log('Client :: connect');
        })

        client.on('end', () => {
            console.log('Client :: end');
        })

        addNewInstance("shit",client);

        yield put(connectDBDone(client));
    } catch (ex) {
        yield put(connectDBError(ex));
        console.log(ex);
    }
}

export function* disconnectDBInstance(instance) {
    try {
        instance && instance.quit();
    } catch (ex) {
        console.log(ex);
    }
}

export function* removeDB(name, instance) {
    deleteInstance(name)
}