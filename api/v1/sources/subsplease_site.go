package sources

import (
	"io"
	"net/http"

	"github.com/troytabrilla/oshirase/api/api/conf"
)

type SubsPleaseSite struct {
	Config *conf.Config
}

func (site *SubsPleaseSite) FetchLatest() ([]byte, error) {
	res, err := http.Get(site.Config.SubsPlease.RSS.URL)
	if err != nil {
		return []byte{}, err
	}

	defer res.Body.Close()

	body, err := io.ReadAll(res.Body)
	if err != nil {
		return []byte{}, err
	}

	return body, nil
}

func (site *SubsPleaseSite) FetchSchedule() ([]byte, error) {
	// TODO Implement
	return []byte{}, nil
}
