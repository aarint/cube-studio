import { takeLatest } from 'redux-saga/effects';
import { CONNECT_DB, DISCONNECT_DB, } from "../actions/Connect";
import { INSTANCE_ADD, INSTANCE_DELETE, INSTANCE_GET } from "../actions/Instance";
import { STRING_DOING, STRING_ADD, STRING_GET, STRING_DELETE, KEYS } from "../actions/Redis";
import { connectDBInstance, disconnectDBInstance } from './Connect';
import { addConnectedInstance, getConnectedInstance, deleteConnectedInstance, getConnectedConfig } from './Instance';
import { doingString, addStringByKeyValue, deleteStringByKey, getStringByKey, getCurrentInstanceKeys } from './Redis';

export default function* index() {
    yield [
        takeLatest(CONNECT_DB, connectDBInstance),
        takeLatest(DISCONNECT_DB, disconnectDBInstance),

        takeLatest(INSTANCE_ADD, addConnectedInstance),
        takeLatest(INSTANCE_DELETE, deleteConnectedInstance),
        takeLatest(INSTANCE_GET, getConnectedInstance),

        takeLatest(KEYS, getCurrentInstanceKeys),
        
        // takeLatest(STRING_DOING, doingString),
        // takeLatest(STRING_ADD, addStringByKeyValue),
        // takeLatest(STRING_DELETE, deleteStringByKey),
        // takeLatest(STRING_GET, getStringByKey)
    ]
}