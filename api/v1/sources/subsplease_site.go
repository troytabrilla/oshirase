package sources

import (
	"github.com/troytabrilla/oshirase/api/api/conf"
)

type SubsPleaseSite struct {
	Config *conf.Config
}

func (site SubsPleaseSite) FetchLatest([]byte, error) {
	// TODO Implement
}

func (site SubsPleaseSite) FetchSchedule([]byte, error) {
	// TODO Implement
}
