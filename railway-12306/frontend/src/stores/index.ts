import { defineStore } from 'pinia'

export const useHomeStore = defineStore('home', {
  state: () => ({
    departureStation: '北京',
    arrivalStation: '上海',
    departDate: null as Date | null,
    recentSearches: [] as string[],
  }),
  
  actions: {
    switchStations() {
      const temp = this.departureStation
      this.departureStation = this.arrivalStation
      this.arrivalStation = temp
    },
    
    setDepartDate(date: Date) {
      this.departDate = date
    },
    
    addRecentSearch(search: string) {
      if (!this.recentSearches.includes(search)) {
        this.recentSearches.unshift(search)
        if (this.recentSearches.length > 10) {
          this.recentSearches.pop()
        }
      }
    }
  }
})