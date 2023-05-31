package models

import (
	"encoding/xml"
	"strings"
	"time"

	"github.com/troytabrilla/oshirase/api/api/conf"
	"github.com/troytabrilla/oshirase/api/api/v1/sources"
)

type PubDate struct {
	time.Time
}

func (pb *PubDate) UnmarshalXML(decoder *xml.Decoder, start xml.StartElement) error {
	layout := "Mon, 02 Jan 2006 15:04:05 +0000"

	var v string
	decoder.DecodeElement(&v, &start)
	parse, err := time.Parse(layout, v)
	if err != nil {
		return err
	}
	*pb = PubDate{parse}

	return nil
}

type SubsPleaseLatest struct {
	Title    string  `xml:"title"`
	Link     string  `xml:"link"`
	Category string  `xml:"category"`
	PubDate  PubDate `xml:"pubDate"`
}

type SubsPleaseChannel struct {
	Items []SubsPleaseLatest `xml:"item"`
}
type SubsPleaseRSS struct {
	Channel SubsPleaseChannel `xml:"channel"`
}

type SubsPleaseResult struct {
	XMLName xml.Name          `xml:"rss"`
	Version string            `xml:"version,attr"`
	Channel SubsPleaseChannel `xml:"channel"`
}

type SubsPlease struct {
	Config *conf.Config
}

func (sp *SubsPlease) FetchLatest() (map[string]SubsPleaseLatest, error) {
	site := sources.SubsPleaseSite{Config: sp.Config}
	latest := make(map[string]SubsPleaseLatest)

	body, err := site.FetchLatest()
	if err != nil {
		return latest, err
	}

	var result SubsPleaseResult
	err = xml.Unmarshal(body, &result)
	if err != nil {
		return latest, err
	}

	for _, item := range result.Channel.Items {
		title := strings.Split(item.Category, " - ")
		latest[title[0]] = item
	}

	return latest, nil
}
