package importer

import (
	"bytes"
	"cmp"
	"encoding/json"
	"strings"
)

// Everything the importer keeps of a Commonly answer is one of these structs. Decoding drops every field not named here, so an
// email, a token, a webhook url or any field a later Commonly version adds cannot pass through: the structs are the allowlist.

// Me is the caller, from GET /api/auth/user. The email in that answer is not decoded.
type Me struct {
	ID    string `json:"_id"`
	Role  string `json:"role"`
	IsBot bool   `json:"isBot"`
}

// Member is one pod member as the pod routes populate it: an id, a name, an avatar and whether it is an agent.
type Member struct {
	ID          string
	Username    string
	Avatar      string
	IsBot       bool
	DisplayName string // botMetadata.displayName: an agent's own name
	InstanceID  string
}

// UnmarshalJSON takes a populated member or a bare id.
func (m *Member) UnmarshalJSON(b []byte) error {
	if b = bytes.TrimSpace(b); len(b) > 0 && b[0] == '"' {
		return json.Unmarshal(b, &m.ID)
	}
	var v struct {
		ID       string `json:"_id"`
		Username string `json:"username"`
		Avatar   string `json:"profilePicture"`
		IsBot    bool   `json:"isBot"`
		Meta     *struct {
			DisplayName string `json:"displayName"`
			InstanceID  string `json:"instanceId"`
		} `json:"botMetadata"`
	}
	if err := json.Unmarshal(b, &v); err != nil {
		return err
	}
	*m = Member{ID: v.ID, Username: v.Username, Avatar: v.Avatar, IsBot: v.IsBot}
	if v.Meta != nil {
		m.DisplayName, m.InstanceID = v.Meta.DisplayName, v.Meta.InstanceID
	}
	return nil
}

// Label is what people see for an agent.
func (m Member) Label() string {
	for _, v := range []string{m.DisplayName, m.Username, m.ID} {
		if v != "" {
			return v
		}
	}
	return "agent"
}

// Pod is a pod as listed. Its tasks, announcements, parent and last message are not decoded.
type Pod struct {
	ID      string   `json:"_id"`
	Name    string   `json:"name"`
	Type    string   `json:"type"`
	Members []Member `json:"members"`
}

// Has reports whether id is among the pod's members.
func (p Pod) Has(id string) bool {
	for _, m := range p.Members {
		if m.ID != "" && m.ID == id {
			return true
		}
	}
	return false
}

// Message is one message as listPodMessages answers it. Both stores give one shape, with camel and snake keys both present, so only
// one spelling is decoded. Reactions, replies, receipts and payloads are not decoded, except the names of attachments.
type Message struct {
	ID       string          `json:"id"`
	Content  string          `json:"content"`
	Type     string          `json:"messageType"`
	At       string          `json:"createdAt"`
	Username string          `json:"username"`
	UserID   string          `json:"user_id"`
	IsBot    bool            `json:"is_bot"`
	Author   json.RawMessage `json:"userId"`
	Payload  json.RawMessage `json:"payload"`
	Attach   json.RawMessage `json:"attachments"`
}

// Norm is a message with its fields settled: one id, one time, one author.
type Norm struct {
	ID, Text, At, Author, AuthorID string
	AuthorBot                      bool
	Files                          []string
	System                         bool
}

// Normalize settles the author, which the route gives as user_id/username and again as userId (an object or a bare id).
func (m Message) Normalize() Norm {
	n := Norm{ID: m.ID, Text: m.Content, At: canonTime(m.At), Author: m.Username, AuthorID: m.UserID, AuthorBot: m.IsBot, System: m.Type == "system"}
	var a struct {
		ID       string `json:"_id"`
		Username string `json:"username"`
		IsBot    bool   `json:"isBot"`
	}
	if json.Unmarshal(m.Author, &a) == nil && a.ID != "" {
		n.AuthorID, n.Author, n.AuthorBot = cmp.Or(n.AuthorID, a.ID), cmp.Or(n.Author, a.Username), n.AuthorBot || a.IsBot
	} else {
		var s string
		if json.Unmarshal(m.Author, &s) == nil {
			n.AuthorID = cmp.Or(n.AuthorID, s)
		}
	}
	n.Files = append(fileNames(m.Attach), payloadFiles(m.Payload)...)
	n.Author = strings.TrimSpace(cmp.Or(n.Author, "unknown"))
	return n
}

type file struct {
	Original string `json:"originalName"`
	FileName string `json:"fileName"`
	Name     string `json:"name"`
}

func fileNames(raw json.RawMessage) []string {
	var fs []file
	if json.Unmarshal(raw, &fs) != nil {
		return nil
	}
	var out []string
	for _, f := range fs {
		out = append(out, cmp.Or(f.Original, f.FileName, f.Name, "file"))
	}
	return out
}

func payloadFiles(raw json.RawMessage) []string {
	var p struct {
		Attachments json.RawMessage `json:"attachments"`
	}
	if json.Unmarshal(raw, &p) != nil {
		return nil
	}
	return fileNames(p.Attachments)
}

// Grant is a connector grant as the owner's list carries it, reduced to a connector, an access level and a pod.
type Grant struct {
	Connector json.RawMessage `json:"connector"`
	WriteMode string          `json:"writeMode"`
	Target    struct {
		Kind string `json:"kind"`
		ID   string `json:"id"`
	} `json:"target"`
	RevokedAt *string `json:"revokedAt"`
}

// ConnectorID is the connector's id, whether the answer carries an object or a bare name.
func (g Grant) ConnectorID() string {
	var o struct {
		ID string `json:"id"`
	}
	if json.Unmarshal(g.Connector, &o) == nil && o.ID != "" {
		return strings.ToLower(o.ID)
	}
	var s string
	_ = json.Unmarshal(g.Connector, &s)
	return strings.ToLower(s)
}

// Integration is an integration row reduced to its type, status and creation time. Its config, where the tokens and webhook urls
// live, is not decoded.
type Integration struct {
	Type      string `json:"type"`
	Status    string `json:"status"`
	CreatedAt string `json:"createdAt"`
}
