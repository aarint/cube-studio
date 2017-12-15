import { Client } from 'ssh2';
import net from 'net';

export function* connectToDB() {
    try {
        const conn = new Client();

        conn.on('ready', () => {
            console.log('Client :: ready');
            conn.shell(function(err, stream) {
              if (err) throw err;
              stream.on('close', function() {
                console.log('Stream :: close');
                conn.end();
              }).on('data', function(data) {
                console.log('STDOUT: ' + data);
              }).stderr.on('data', function(data) {
                console.log('STDERR: ' + data);
              });
              stream.end('ls -l\nexit\n');
            });
        }).connect({
            host: '10.2.1.128',
            port: 22
        })



    } catch (ex) {
        console.log(ex);
    }

}