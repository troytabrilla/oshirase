package models

import (
	"testing"

	"github.com/troytabrilla/oshirase/api/api/conf"
	_ "github.com/troytabrilla/oshirase/api/api/test"
)

func TestSubsPleaseFetchLatest(test *testing.T) {
	config := conf.LoadConfig()
	subsplease := SubsPlease{Config: &config}

	latest, err := subsplease.FetchLatest()
	if len(latest) == 0 || err != nil {
		test.Fatalf(`FetchLatest should return a non-empty map of latst anime.`)
	}

}
