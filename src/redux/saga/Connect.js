import Redis from 'redis';
import {addInstance,deleteInstance} from './Instance';

export function* connectDB(name) {
    try {
        let client = Redis.createClient(6379, '10.2.1.128');

        client.on('connect', () => {
            console.log('Client :: connect');
            addInstance(name,client);
        })

        client.on('end', () => {
            console.log('Client :: end');
        })

        return client;
    } catch (ex) {
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

export function* removeDB(name,instance){
    deleteInstance(name)
}