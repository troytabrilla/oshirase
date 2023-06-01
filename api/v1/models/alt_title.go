package models

import (
	"context"

	"github.com/troytabrilla/oshirase/api/api/conf"
	"go.mongodb.org/mongo-driver/bson"
	"go.mongodb.org/mongo-driver/mongo"
)

type AltTitle struct {
	Config *conf.Config
	Client *mongo.Client
}

func (alt *AltTitle) FetchAltTitles() (map[int]AltTitles, error) {
	collection := alt.Client.Database(alt.Config.DB.MongoDB.Database).Collection("alt_titles")
	keys := bson.D{{Key: "media_id", Value: 1}}
	indexModel := mongo.IndexModel{
		Keys: keys,
	}

	_, err := collection.Indexes().CreateOne(context.TODO(), indexModel)
	if err != nil {
		return map[int]AltTitles{}, err
	}

	results, err := collection.Find(context.TODO(), bson.M{})
	if err != nil {
		return map[int]AltTitles{}, err
	}

	var decoded []AltTitles
	err = results.All(context.TODO(), &decoded)
	if err != nil {
		return map[int]AltTitles{}, err
	}

	final := make(map[int]AltTitles)
	for _, item := range decoded {
		final[item.MediaID] = item
	}

	return final, nil
}
