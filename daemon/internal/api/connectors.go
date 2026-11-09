package api

import (
	"fmt"
	"net/http"
	"net/url"

	"github.com/Team-Commonly/jellymoon/daemon/internal/protocol"
)

// The calls `jellymoon connector` makes as the person (ADR 0009): the existing connect, grant, approvers, list and receipts
// routes, under their jmu_ token. The upstream credential travels in Connect's body and in no other call.

// Connect stores the upstream credential in the pod's vault. The answer never carries it.
func (c *Client) Connect(userToken, podID string, in protocol.ConnectRequest) (protocol.Connection, error) {
	var out protocol.Connection
	return out, c.post(fmt.Sprintf("/v1/pods/%s/connections", url.PathEscape(podID)), userToken, in, &out)
}

// Grant lets one agent use one connection.
func (c *Client) Grant(userToken, podID string, in protocol.GrantRequest) (protocol.Grant, error) {
	var out protocol.Grant
	return out, c.post(fmt.Sprintf("/v1/pods/%s/grants", url.PathEscape(podID)), userToken, in, &out)
}

// SetApprovers replaces a connection's approver set; an empty list clears it.
func (c *Client) SetApprovers(userToken, podID string, in protocol.ApproversRequest) (protocol.Approvers, error) {
	var out protocol.Approvers
	return out, c.post(fmt.Sprintf("/v1/pods/%s/approvers", url.PathEscape(podID)), userToken, in, &out)
}

// Connectors lists the pod's connections with their pinned operations.
func (c *Client) Connectors(userToken, podID string) ([]protocol.ConnectorView, error) {
	var out protocol.ConnectorListOutput
	err := c.call(http.MethodGet, fmt.Sprintf("/v1/pods/%s/connectors", url.PathEscape(podID)), userToken, nil, &out)
	return out.Connections, err
}

// ConnectorReceipts reads the server's receipts, newest first, optionally of one write's decision.
func (c *Client) ConnectorReceipts(userToken, podID string, decision *protocol.DecisionID, limit int) ([]protocol.ConnectorReceipt, error) {
	q := url.Values{}
	if decision != nil {
		q.Set("decisionId", *decision)
	}
	if limit > 0 {
		q.Set("limit", fmt.Sprint(limit))
	}
	path := fmt.Sprintf("/v1/pods/%s/connectors/receipts", url.PathEscape(podID))
	if len(q) > 0 {
		path += "?" + q.Encode()
	}
	var out protocol.ConnectorReceiptsOutput
	err := c.call(http.MethodGet, path, userToken, nil, &out)
	return out.Receipts, err
}
