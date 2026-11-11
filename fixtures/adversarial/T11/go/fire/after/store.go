package store

type Store interface {
	Count() int
}

type Report struct {
	Total int
}

func Summarise(store Store) Report {
	return Report{Total: store.Count()}
}
