package models

import (
	"context"
	"reflect"
	"testing"

	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/db"
	_ "github.com/troytabrilla/oshirase/api/api/test"
	"go.mongodb.org/mongo-driver/bson"
)

func TestFetchAltTitles(test *testing.T) {
	config := conf.LoadConfig()
	mongodb := db.MongoDB{Config: &config}
	client := mongodb.GetClient()
	defer mongodb.CloseClient(client)

	collection := client.Database(config.DB.MongoDB.Database).Collection("alt_titles")
	err := collection.Drop(context.TODO())
	if err != nil {
		test.Fatalf("Could not reset collection: %v", err)
	}

	_, err = collection.InsertOne(context.TODO(), bson.D{{Key: "media_id", Value: 1}, {Key: "alt_titles", Value: bson.A{"gintama"}}})
	if err != nil {
		test.Fatalf("Could not insert fixture: %v", err)
	}

	alt_title := AltTitle{Config: &config, Client: client}
	actual, err := alt_title.FetchAltTitles()
	expected := map[int]AltTitles{1: {MediaID: 1, AltTitles: []string{"gintama"}}}

	if !reflect.DeepEqual(actual, expected) || err != nil {
		test.Fatalf("FetchAltTitles should return a map of alt titles: %v %v %v", actual, expected, err)
	}
}
