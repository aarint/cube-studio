import { combineReducers } from 'redux';
import { handleConnection } from './Connect';
import { handleInstance } from './Instance';
import { handleRedis } from './Redis';
import memcached from './Memcached';

export default combineReducers({
    handleConnection,
    handleInstance,
    handleRedis,
    memcached
});
