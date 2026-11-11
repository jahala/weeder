package service

const Service = "build.example.app"

func Endpoint(host string) string {
	return "https://" + host + "/"
}
