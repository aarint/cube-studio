import { combineReducers } from 'redux';
import { handleConnection } from './Connect';
import { handleInstance } from './Instance';
import { handleRedis } from './Redis';

export default combineReducers({
    handleConnection,
    handleInstance,
    handleRedis
})