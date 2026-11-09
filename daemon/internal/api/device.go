package api

import "github.com/Team-Commonly/jellymoon/daemon/internal/protocol"

// StartDevice starts a device sign-in. It sends no token.
func (c *Client) StartDevice() (protocol.DeviceStartResponse, error) {
	var out protocol.DeviceStartResponse
	return out, c.post("/v1/auth/device", "", struct{}{}, &out)
}

// DeviceToken polls a device code once. Before approval the error is an *Error with code authorization_pending or slow_down.
func (c *Client) DeviceToken(deviceCode string) (protocol.DeviceTokenResponse, error) {
	var out protocol.DeviceTokenResponse
	return out, c.post("/v1/auth/device/token", "", protocol.DeviceTokenRequest{DeviceCode: deviceCode}, &out)
}
