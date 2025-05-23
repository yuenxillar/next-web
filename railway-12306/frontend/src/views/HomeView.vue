<template>
  <div class="railway-app">
    <!-- 顶部导航栏 -->
    <a-layout-header class="header">
      <div class="logo-container">
        <img src="@/assets/railway-logo.png" alt="铁路logo" class="logo" />
        <div class="logo-text">
          <div class="main-title">中国铁路12306</div>
          <div class="sub-title">12306 CHINA RAILWAY</div>
        </div>
      </div>
      <a-input-search placeholder="搜索车票、餐饮、旅游、相关问答" class="search-input" :style="{ width: '300px' }">
        <template #enterButton>
          <a-button type="primary">
            <search-outlined />
          </a-button>
        </template>
      </a-input-search>
      <div class="header-right">
        <a-space>
          <a href="#">无障碍</a>
          <a-divider type="vertical" />
          <a href="#">简体版</a>
          <a-divider type="vertical" />
          <a href="#">English</a>
          <a-divider type="vertical" />
          <a href="#">我的12306</a>
          <a-divider type="vertical" />
          <a href="#">登录</a>
          <a href="#">注册</a>
        </a-space>
      </div>
    </a-layout-header>

    <!-- 主导航菜单 -->
    <a-menu mode="horizontal" class="main-menu">
      <a-menu-item key="home" class="menu-item">首页</a-menu-item>
      <a-menu-item key="ticket" class="menu-item">
        车票
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="team" class="menu-item">
        团购服务
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="member" class="menu-item">
        会员服务
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="station" class="menu-item">
        站车服务
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="business" class="menu-item">
        商旅服务
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="travel" class="menu-item">
        出行指南
        <down-outlined />
      </a-menu-item>
      <a-menu-item key="info" class="menu-item">
        信息查询
        <down-outlined />
      </a-menu-item>
    </a-menu>

    <!-- 主体内容 -->
    <div class="main-content">
      <!-- 查询窗口 -->
      <div class="query-container">
        <div class="query-tabs">
          <a-tabs default-active-key="1">
            <a-tab-pane key="1">
              <template #tab>
                <span class="tab-icon"><car-outlined /> 车票</span>
              </template>
              <a-card class="query-card">
                <a-radio-group v-model:value="ticketType" button-style="solid" class="query-type">
                  <a-radio-button value="a">单程</a-radio-button>
                  <a-radio-button value="b">往返</a-radio-button>
                  <a-radio-button value="c">中转换乘</a-radio-button>
                  <a-radio-button value="d">退改签</a-radio-button>
                </a-radio-group>

                <div class="form-item">
                  <div class="label">出发地</div>
                  <a-input placeholder="简拼/全拼/汉字" @focus="hanleFirstInputFocus">
                    <template #suffix>
                      <environment-outlined />
                    </template>
                  </a-input>
                </div>

                <div class="form-item">
                  <div class="label">到达地</div>
                  <a-input placeholder="简拼/全拼/汉字" @focus="hanleFirstInputFocus">
                    <template #suffix>
                      <environment-outlined />
                    </template>
                  </a-input>
                </div>

                <div class="form-item">
                  <div class="label">出发日期</div>
                  <a-date-picker v-model="departureDate" style="width: 100%" :locale="locale" placeholder="请选择日期" />
                </div>

                <div class="form-item">
                  <a-checkbox>学生</a-checkbox>
                  <a-checkbox>高铁/动车</a-checkbox>
                </div>

                <a-button type="primary" block class="query-btn">查询</a-button>
              </a-card>
            </a-tab-pane>
            <a-tab-pane key="2">
              <template #tab>
                <span class="tab-icon"><question-circle-outlined /> 常用查询</span>
              </template>
            </a-tab-pane>
            <a-tab-pane key="3">
              <template #tab>
                <span class="tab-icon"><calendar-outlined /> 订餐</span>
              </template>
            </a-tab-pane>
          </a-tabs>
        </div>
      </div>

      <!-- 轮播图 -->
      <div class="banner-container">
        <a-carousel autoplay :after-change="onChange">
          <div v-for="(banner, index) in banners" :key="index">
            <img :src="banner" class="banner-img" />
          </div>
        </a-carousel>
      </div>
    </div>

    <!-- 快捷服务图标 -->
    <div class="quick-services">
      <div class="service-item" @click="goToService('special-passenger')">
        <team-outlined class="service-icon" />
        <div>重点旅客预约</div>
      </div>
      <div class="service-item" @click="goToService('lost-property')">
        <inbox-outlined class="service-icon" />
        <div>遗失物品查找</div>
      </div>
      <div class="service-item" @click="goToService('car-service')">
        <car-outlined class="service-icon" />
        <div>约车服务</div>
      </div>
      <div class="service-item" @click="goToService('consignment')">
        <compass-outlined class="service-icon" />
        <div>便民托运</div>
      </div>
      <div class="service-item" @click="goToService('station-guide')">
        <environment-outlined class="service-icon" />
        <div>车站引导</div>
      </div>
      <div class="service-item" @click="goToService('station-style')">
        <bank-outlined class="service-icon" />
        <div>站车风采</div>
      </div>
      <div class="service-item" @click="goToService('feedback')">
        <user-outlined class="service-icon" />
        <div>用户反馈</div>
      </div>
    </div>

    <!-- 服务卡片区域 -->
    <div class="service-cards">
      <a-row :gutter="16">
        <a-col :span="12">
          <a-card class="service-card member-card">
            <div class="card-content">
              <div class="card-title">
                <h2>会员服务</h2>
                <p>铁路畅行 尊享体验</p>
                <p>12306铁路会员积分服务</p>
              </div>
              <div class="card-image">
                <img src="@/assets/member-train.png" alt="会员服务" />
              </div>
            </div>
          </a-card>
        </a-col>
        <a-col :span="12">
          <a-card class="service-card food-card">
            <div class="card-content">
              <div class="card-title">
                <h2>餐饮·特产</h2>
                <p>带有温度的旅途配餐</p>
                <p>享受星级的体验和丰富的味道</p>
              </div>
              <div class="card-image">
                <img src="@/assets/food.png" alt="餐饮特产" />
              </div>
            </div>
          </a-card>
        </a-col>
      </a-row>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import zhCN from 'ant-design-vue/es/locale/zh_CN';
import {
  SearchOutlined,
  DownOutlined,
  CarOutlined,
  QuestionCircleOutlined,
  CalendarOutlined,
  TeamOutlined,
  InboxOutlined,
  CompassOutlined,
  EnvironmentOutlined,
  BankOutlined,
  UserOutlined
} from '@ant-design/icons-vue';


const ticketType = ref('a');
const locale = zhCN;
const departureDate = ref<any>();
const banners = [
  '/banner10.jpg',
  '/banner12.jpg',
  '/banner26.jpg',
  '/banner0619.jpg',
  '/banner20200707.jpg',
  '/banner20201223.jpg',
];


// banner 切换事件处理函数
const onChange = (current: number) => {
};

const hanleFirstInputFocus = () => {
  console.log('focus');
}

const goToService = (serviceType: string) => {
  const uris = {
    'special-passenger': 'view/icentre_qxyyInfo.html',
    'lost-property': 'view/icentre_lostInfo.html',
    'car-service': 'view/station/shared_Car.html',
    'consignment': 'view/station/hand.html',
    'station-guide': 'czyd_2143/',
    'station-style': 'zcfc_2548/',
    'feedback': 'view/advice_adviceInfo.html'
  };
  // 为了避免类型错误，先检查 serviceType 是否是 uris 对象的有效键
  // @ts-ignore
  const url = `https://www.12306.cn/index/${uris[serviceType]}`;
  window.open(url, '_blank');
}
</script>

<style scoped>
.railway-app {
  font-family: 'Microsoft YaHei', Arial, sans-serif;
  width: 100%;
  min-height: 100vh;
  margin: 0;
  padding: 0;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  align-items: center;
  background-color: white;
}

/* 添加全局重置样式 */
html,
body {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  overflow-x: hidden;
}

/* 修改 header 样式确保宽度 100% */
.header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 60px;
  padding: 0 20px;
  background-color: white;
  width: 70%;
}

.logo-container {
  display: flex;
  align-items: center;
}

.logo {
  width: 40px;
  height: 40px;
  margin-right: 10px;
}

.logo-text {
  display: flex;
  flex-direction: column;
}

.main-title {
  font-size: 16px;
  font-weight: bold;
  color: #333;
}

.sub-title {
  font-size: 12px;
  color: #999;
}

.header-right {
  font-size: 12px;
}

/* 确保菜单宽度 100% */
.main-menu {
  background-color: #4a90e2;
  color: white;
  display: flex;
  justify-content: center;
  width: 100%;
}

.menu-item {
  color: white !important;
  font-size: 15px;
  padding-right: 100px;
}

/* 确保内容区域宽度 100% */
.main-content {
  display: flex;
  min-height: 360px;
  width: 100%;
}

.query-container {
  position: absolute;
  width: 500px;
  padding: 20px;
  z-index: 10;
  left: 20vw;
  max-height: 160px;
}

.query-tabs {
  background-color: white;
  border-radius: 4px;
  align-items: center;
  justify-items: center;
}

.tab-icon {
  display: flex;
  align-items: center;
  gap: 5px;
}

.query-card {
  border: none;
  box-shadow: none;
}

.query-type {
  margin-bottom: 15px;
  width: 100%;
  display: flex;
}

.form-item {
  margin-bottom: 15px;
}

.label {
  margin-bottom: 5px;
  font-size: 14px;
  justify-self: start;
}

.query-btn {
  height: 40px;
  font-size: 16px;
}

.banner-container {
  flex: 1;
  position: relative;
  width: 100%;
  height: 100%;
  min-height: 30vh;
}

.banner {
  height: 100%;
  width: 100%;
}

.banner-img {
  width: 100%;
  height: 100%;
  min-height: 450px;
  object-fit: cover;
  transition: all 0.5s ease;
}

.province {
  font-size: 80px;
  margin-bottom: 10px;
}

.slogan {
  font-size: 36px;
  margin-bottom: 10px;
}

.description {
  font-size: 24px;
}

.carousel-indicators {
  position: absolute;
  bottom: 20px;
  width: 100%;
  text-align: center;
}

.quick-services {
  display: flex;
  justify-content: space-around;
  padding: 20px 0;
  background-color: white;
  border-bottom: 1px solid #eee;
  width: 80%;
  min-height: 10vh;
}

.service-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  font-size: 14px;
  color: #666;
  cursor: pointer;
}

.service-icon {
  font-size: 24px;
  color: #4a90e2;
  margin-bottom: 8px;
}

.service-cards {
  padding: 20px;
  background-color: #f5f5f5;
}

.service-card {
  margin-bottom: 20px;
  border-radius: 8px;
  overflow: hidden;
}

.card-content {
  display: flex;
  justify-content: space-between;
  padding: 20px;
}

.card-title {
  flex: 1;
}

.card-title h2 {
  font-size: 24px;
  color: #4a90e2;
  margin-bottom: 10px;
}

.card-title p {
  color: #666;
  margin-bottom: 5px;
}

.card-image {
  flex: 1;
  text-align: right;
}

.card-image img {
  max-width: 100%;
  max-height: 120px;
}

.member-card {
  background-color: #e8f4ff;
}

.food-card {
  background-color: #f0f9eb;
}
</style>