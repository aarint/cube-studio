/**
 * Main page show the all connections that include saved in storage.
 * Click one will show the DB instance in the opened tab window.
 */

import React from 'react';
import { connect } from 'react-redux';
import { Modal, Button, Layout,Input } from 'antd';
import Instance from './Instance';
import { connectDB } from '../redux/actions/Connect';
import { getSavedInstances } from '../redux/actions/Instance';

const { Content, Sider } = Layout;

class Welcome extends React.PureComponent {

    state = {
        visible: false
    }

    /**
     * Open connection dialog. Connect the DB after input the connection config.
     * Then create a new tab that contain the DB instance.
     */
    openConnection = () => {
        this.setState({ visible: true })

        //dialog.showOpenDialog({ properties: ['openFile', 'openDirectory', 'multiSelections'] })
    }

    handleConnect = () => {
        this.props.connectDB({ name: 'shit', ip: 'localhost', port: 6379, password: 'shit' });
        this.props.addInstance('localhost');
    }

    render() {
        console.log(this.props.result);

        return (
            <Layout>
                <Content style={{ marginLeft: '200px', overflow: 'initial' }}>
                    <div style={{ padding: 0, background: '#fff', textAlign: 'center' }}>
                        <div></div>
                        <Button type="primary" onClick={this.openConnection}>open connect dialog</Button>
                    </div>
                </Content>

                {
                    this.state.visible &&
                    <div>
                        <Input />
                        <Button onClick={this.handleConnect}>connect</Button>
                    </div>
                }

                {

                }
            </Layout>
        )
    }

    componentDidMount() {
        this.props.getSavedInstances();
    }
}

function mapStateToProps(state) {
    console.log(state);

    return {
        instances: state.handleInstance.instances || null,
        result: state.handleConnection.result || null
    }
}

export default connect(mapStateToProps, { connectDB, getSavedInstances })(Welcome)