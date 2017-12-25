import { put, call } from 'redux-saga/effects';
import Redis from 'redis';
import { connectStart, connectDone, connectError } from '../actions/Connect';

export function* connectDB(name) {
    try {
        yield put(connectStart());

        let client = Redis.createClient(6379, '10.2.1.128');

        client.on('connect', () => {
            console.log('Client :: connect');
        })

        client.on('end', () => {
            console.log('Client :: end');
        })

        yield put(connectDone(client));
    } catch (ex) {
        yield put(connectError(ex));
        console.log(ex);
    }
}

export function* disconnectDB(instance) {
    try {
        instance && instance.quit();
    } catch (ex) {
        console.log(ex);
    }
}

export function* removeDB(name, instance) {
    deleteInstance(name)
}