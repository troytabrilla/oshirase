package db

import (
	"context"
	"log"
	"os"

	"github.com/troytabrilla/oshirase/api/api/conf"
	"go.mongodb.org/mongo-driver/mongo"
	"go.mongodb.org/mongo-driver/mongo/options"
)

type MongoDB struct {
	Config *conf.Config
}

func (mongodb MongoDB) GetClient() *mongo.Client {
	uri := mongodb.Config.DB.MongoDB.URI

	serverAPI := options.ServerAPI(options.ServerAPIVersion1)
	opts := options.Client().ApplyURI(uri).SetServerAPIOptions(serverAPI)
	client, err := mongo.Connect(context.TODO(), opts)
	if err != nil {
		log.Fatalf("Could not connect to MongoDB: %v.", err)
		os.Exit(1)
	}

	return client
}

func (mongo MongoDB) CloseClient(client *mongo.Client) {
	if err := client.Disconnect(context.TODO()); err != nil {
		os.Exit(1)
	}
}
