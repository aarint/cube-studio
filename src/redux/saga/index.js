import { takeLatest } from 'redux-saga/effects';
import { CONNECT_DB, DISCONNECT_DB } from "../actions/Connect";
import { INSTANCE_ADD, INSTANCE_DELETE, INSTANCES_SAVED_GET } from "../actions/Instance";
import { STRING_DOING, STRING_ADD, STRING_GET, STRING_DELETE, KEYS_GET, KEY_VALUE_GET, CONFIG_GET, CONFIG_SET } from "../actions/Redis";
import { connectDB, disconnectDB } from './Connect';
import { addConnectedInstance, getAllSavedInstances, deleteConnectedInstance, getConnectedConfig } from './Instance';
import { doingString, addStringByKeyValue, deleteStringByKey, getObjectByKey, getCurrentInstanceKeys, getConfigByKey, setConfigByKey } from './Redis';

export default function* index() {
    yield [
        takeLatest(CONNECT_DB, connectDB),
        takeLatest(DISCONNECT_DB, disconnectDB),

        takeLatest(INSTANCE_ADD, addConnectedInstance),
        takeLatest(INSTANCE_DELETE, deleteConnectedInstance),
        takeLatest(INSTANCES_SAVED_GET, getAllSavedInstances),

        takeLatest(KEYS_GET, getCurrentInstanceKeys),

        takeLatest(STRING_DOING, doingString),
        takeLatest(STRING_ADD, addStringByKeyValue),
        takeLatest(STRING_DELETE, deleteStringByKey),

        takeLatest(CONFIG_GET, getConfigByKey),
        takeLatest(CONFIG_SET, setConfigByKey),

        takeLatest(KEY_VALUE_GET, getObjectByKey)
    ]
}